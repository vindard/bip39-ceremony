#!/usr/bin/env python3
"""Drive the BIP-39 ceremony TUI in a PTY and render a colored PNG screenshot."""
from __future__ import annotations
import fcntl, os, pty, struct, sys, termios, time, select, json
import pyte
from PIL import Image, ImageDraw, ImageFont, ImageChops

# Monospace font paths. Override with SHOT_FONT_REG / SHOT_FONT_BOLD, or find
# yours with: fc-match -f '%{file}\n' 'JetBrainsMono Nerd Font Mono:style=Regular'
REG = os.environ.get(
    "SHOT_FONT_REG", "/usr/share/fonts/TTF/JetBrainsMonoNerdFontMono-Regular.ttf"
)
BOLD = os.environ.get(
    "SHOT_FONT_BOLD", "/usr/share/fonts/TTF/JetBrainsMonoNerdFontMono-Bold.ttf"
)

NAMED = {
    "black": "#1a1a1a", "red": "#cc3333", "green": "#33aa55", "brown": "#b58900",
    "blue": "#268bd2", "magenta": "#d33682", "cyan": "#2aa198", "white": "#d0d0d0",
    "brightblack": "#555555", "brightred": "#ff5f5f", "brightgreen": "#5fff87",
    "brightyellow": "#ffff87", "brightblue": "#5fafff", "brightmagenta": "#ff87ff",
    "brightcyan": "#5fffff", "brightwhite": "#ffffff",
}
DEFAULT_FG = "#d5d5d5"
DEFAULT_BG = "#0c0c0c"

def resolve(c, is_bg):
    if c == "default":
        return DEFAULT_BG if is_bg else DEFAULT_FG
    if c in NAMED:
        return NAMED[c]
    # pyte gives 256/truecolor as 6-hex string
    if len(c) == 6:
        try:
            int(c, 16); return "#" + c
        except ValueError:
            pass
    return DEFAULT_BG if is_bg else DEFAULT_FG

def drive(binary, rows, cols, script):
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, 0, 0))
    import subprocess
    env = dict(os.environ)
    env["TERM"] = "xterm-256color"
    env.pop("NO_COLOR", None)
    env.pop("BIP39_CEREMONY_THEME", None)
    proc = subprocess.Popen([binary], stdin=slave, stdout=slave, stderr=slave,
                            close_fds=True, env=env)
    os.close(slave)
    buf = bytearray()
    def pump(dur):
        end = time.time() + dur
        while time.time() < end:
            r, _, _ = select.select([master], [], [], 0.05)
            if r:
                try:
                    buf.extend(os.read(master, 65536))
                except OSError:
                    return
    pump(0.4)
    for keys, wait in script:
        if keys:
            os.write(master, keys)
        pump(wait)
    snapshot = bytes(buf)  # capture BEFORE the quit sequence
    # graceful quit (q then confirm y)
    try:
        os.write(master, b"q"); pump(0.15)
        os.write(master, b"y"); pump(0.15)
    except OSError:
        pass
    try:
        proc.wait(timeout=2)
    except Exception:
        proc.kill()
    os.close(master)
    return snapshot

def content_bbox(img, margin):
    """Bounding box of non-background ink, expanded by `margin` px and clamped."""
    diff = ImageChops.difference(img, Image.new("RGB", img.size, DEFAULT_BG))
    bb = diff.getbbox()
    if not bb:
        return (0, 0, img.width, img.height)
    l, t, r, b = bb
    return (max(0, l - margin), max(0, t - margin),
            min(img.width, r + margin), min(img.height, b + margin))

VBORDER = set("║│┃ ")  # rows whose only ink is a vertical border are collapsible whitespace

def _collapse(screen, rows, cols):
    """Return the list of source rows to draw, collapsing interior empty bands to one row."""
    def empty(y):
        line = screen.buffer[y]
        return all((line[x].data or " ") in VBORDER for x in range(cols))
    kept, y = [], 0
    while y < rows:
        if empty(y):
            kept.append(y)              # keep one representative of the blank run
            while y < rows and empty(y):
                y += 1
        else:
            kept.append(y)
            y += 1
    return kept

def render(data, rows, cols, out, scale=2, fontsize=17, target_w=1200):
    screen = pyte.Screen(cols, rows)
    stream = pyte.ByteStream(screen)
    stream.feed(data)
    reg = ImageFont.truetype(REG, fontsize * scale)
    bold = ImageFont.truetype(BOLD, fontsize * scale)
    cw = reg.getlength("M")
    ch = int((fontsize * scale) * 1.30)
    kept = _collapse(screen, rows, cols)
    pad = 10 * scale
    W = int(cw * cols) + 2 * pad
    H = ch * len(kept) + 2 * pad
    img = Image.new("RGB", (W, H), DEFAULT_BG)
    d = ImageDraw.Draw(img)
    for row_idx, y in enumerate(kept):
        line = screen.buffer[y]
        for x in range(cols):
            cell = line[x]
            ch_data = cell.data or " "
            fg = resolve(cell.fg, False)
            bg = resolve(cell.bg, True)
            if cell.reverse:
                fg, bg = bg, fg
            px = pad + int(cw * x)
            py = pad + ch * row_idx
            if bg != DEFAULT_BG:
                d.rectangle([px, py, px + int(cw) + 1, py + ch], fill=bg)
            if ch_data != " ":
                f = bold if cell.bold else reg
                d.text((px, py), ch_data, font=f, fill=fg)
    img = img.crop(content_bbox(img, margin=8 * scale))
    W, H = img.size
    if target_w and W > target_w:
        img = img.resize((target_w, round(H * target_w / W)), Image.LANCZOS)
    img.save(out)
    print(f"wrote {out} ({img.size[0]}x{img.size[1]})")

def render_fixed(screen, rows, cols, reg, bold, cw, ch, pad):
    W = int(cw * cols) + 2 * pad
    H = ch * rows + 2 * pad
    img = Image.new("RGB", (W, H), DEFAULT_BG)
    d = ImageDraw.Draw(img)
    for y in range(rows):
        line = screen.buffer[y]
        for x in range(cols):
            cell = line[x]
            ch_data = cell.data or " "
            fg = resolve(cell.fg, False); bg = resolve(cell.bg, True)
            if cell.reverse:
                fg, bg = bg, fg
            px = pad + int(cw * x); py = pad + ch * y
            if bg != DEFAULT_BG:
                d.rectangle([px, py, px + int(cw) + 1, py + ch], fill=bg)
            if ch_data != " ":
                d.text((px, py), ch_data, font=(bold if cell.bold else reg), fill=fg)
    return img

def make_spacer(cols, rows, reg, bold, cw, ch, pad):
    """A synthetic end-card frame that marks the loop boundary."""
    W = int(cw * cols) + 2 * pad
    H = ch * rows + 2 * pad
    img = Image.new("RGB", (W, H), DEFAULT_BG)
    d = ImageDraw.Draw(img)
    AMBER, GOLD, GREY, CORAL = "#ffaf00", "#ffd700", "#9e9e9e", "#ff5f5f"
    # (text, color, bold)
    lines = [
        ("┌───────┐   ┌───────┐   ┌───────┐", AMBER, False),
        ("│ ●   ● │   │ ●   ● │   │ ●     │", AMBER, False),
        ("│   ●   │   │       │   │   ●   │", AMBER, False),
        ("│ ●   ● │   │ ●   ● │   │     ● │", AMBER, False),
        ("└───────┘   └───────┘   └───────┘", AMBER, False),
        ("", GREY, False),
        ("B I P - 3 9   C E R E M O N Y", GOLD, True),
        ("", GREY, False),
        ("dice  →  entropy  →  checksum  →  words", GREY, False),
        ("", GREY, False),
        ("◆ ───────  ▶  demo replays  ───────  ◆", CORAL, True),
    ]
    start_row = (rows - len(lines)) // 2
    for i, (text, color, is_bold) in enumerate(lines):
        if not text:
            continue
        col_off = (cols - len(text)) // 2
        py = pad + ch * (start_row + i)
        f = bold if is_bold else reg
        for j, chn in enumerate(text):
            if chn != " ":
                d.text((pad + int(cw * (col_off + j)), py), chn, font=f, fill=color)
    return img

def animate(binary, rows, cols, steps, out, scale=2, fontsize=14, target_w=820,
            spacer_ms=1700):
    """steps: list of (keys, settle_seconds, hold_ms). One frame captured per step."""
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, 0, 0))
    import subprocess
    env = dict(os.environ); env["TERM"] = "xterm-256color"
    env.pop("NO_COLOR", None); env.pop("BIP39_CEREMONY_THEME", None)
    proc = subprocess.Popen([binary], stdin=slave, stdout=slave, stderr=slave,
                            close_fds=True, env=env)
    os.close(slave)
    buf = bytearray()
    def pump(dur):
        end = time.time() + dur
        while time.time() < end:
            r, _, _ = select.select([master], [], [], 0.03)
            if r:
                try: buf.extend(os.read(master, 65536))
                except OSError: return
    reg = ImageFont.truetype(REG, fontsize * scale)
    bold = ImageFont.truetype(BOLD, fontsize * scale)
    cw = reg.getlength("M"); ch = int((fontsize * scale) * 1.30); pad = 10 * scale
    frames, durations = [], []
    terminal_colors = {DEFAULT_FG, DEFAULT_BG}
    pump(0.4)
    for keys, settle, hold_ms in steps:
        if keys: os.write(master, keys.encode())
        pump(settle)
        screen = pyte.Screen(cols, rows); pyte.ByteStream(screen).feed(bytes(buf))
        for y in range(rows):
            for x in range(cols):
                cell = screen.buffer[y][x]
                terminal_colors.add(resolve(cell.fg, False))
                terminal_colors.add(resolve(cell.bg, True))
        frames.append(render_fixed(screen, rows, cols, reg, bold, cw, ch, pad))
        durations.append(hold_ms)
    try:
        os.write(master, b"q"); pump(0.1); os.write(master, b"y"); pump(0.1)
        proc.wait(timeout=2)
    except Exception:
        proc.kill()
    os.close(master)
    if spacer_ms:
        frames.append(make_spacer(cols, rows, reg, bold, cw, ch, pad))
        durations.append(spacer_ms)
    # Crop every frame by the union content bbox so the card fills the frame
    # evenly (no right-hand black gutter) while all frames stay the same size.
    boxes = [content_bbox(f, margin=8 * scale) for f in frames]
    union = (min(b[0] for b in boxes), min(b[1] for b in boxes),
             max(b[2] for b in boxes), max(b[3] for b in boxes))
    frames = [f.crop(union) for f in frames]
    if target_w and frames[0].width > target_w:
        h = round(frames[0].height * target_w / frames[0].width)
        frames = [f.resize((target_w, h), Image.LANCZOS) for f in frames]
    # One shared palette for all frames, full (non-delta) frames so playback is
    # correct regardless of disposal handling.
    w0, h0 = frames[0].size
    column = Image.new("RGB", (w0, h0 * len(frames)))
    for i, f in enumerate(frames):
        column.paste(f, (0, i * h0))
    anchors = [
        tuple(bytes.fromhex(color.removeprefix("#"))) for color in sorted(terminal_colors)
    ]
    adaptive_count = 256 - len(anchors)
    adaptive = column.quantize(colors=adaptive_count, method=Image.MEDIANCUT)
    adaptive_palette = adaptive.getpalette()[:adaptive_count * 3]
    palette = Image.new("P", (1, 1))
    palette.putpalette([channel for color in anchors for channel in color] + adaptive_palette)
    pal = [f.quantize(palette=palette, dither=Image.NONE) for f in frames]
    pal[0].save(out, save_all=True, append_images=pal[1:], duration=durations,
                loop=0, optimize=False, disposal=1)
    print(f"wrote {out} ({frames[0].width}x{frames[0].height}, {len(frames)} frames)")

if __name__ == "__main__":
    cfg = json.load(open(sys.argv[1]))
    data = drive(cfg["binary"], cfg["rows"], cfg["cols"],
                 [(bytes(k, "utf-8"), w) for k, w in cfg["script"]])
    render(data, cfg["rows"], cfg["cols"], cfg["out"],
           scale=cfg.get("scale", 2), fontsize=cfg.get("fontsize", 17))
