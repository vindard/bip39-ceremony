#!/usr/bin/env python3

import argparse
from pathlib import Path


def extract_method(source: str, signature: str) -> str:
    start = source.index(signature)
    depth = 0
    saw_body = False
    for position in range(start, len(source)):
        character = source[position]
        if character == "{":
            depth += 1
            saw_body = True
        elif character == "}":
            depth -= 1
            if saw_body and depth == 0:
                return source[start : position + 1]
    raise RuntimeError(f"unterminated upstream method: {signature}")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()

    source = Path(args.source)
    view_model = (
        source
        / "app/src/main/java/com/keystone/cold/viewmodel/SetupVaultViewModel.java"
    ).read_text()
    hash_util = (
        source / "app/src/main/java/com/keystone/cold/util/HashUtil.java"
    ).read_text()
    rolling_dice = (
        source
        / "app/src/main/java/com/keystone/cold/ui/fragment/setup/RollingDiceFragment.java"
    ).read_text()
    conversion = extract_method(
        view_model, "public void generateMnemonicFromDiceRolls(byte[] diceRolls)"
    )
    sha256 = extract_method(hash_util, "public static byte[] sha256(String s)")
    completion = extract_method(rolling_dice, "private void onCompleteClick()")
    if "rolls.append(b % 6)" not in conversion:
        raise RuntimeError("unexpected Keystone legacy dice mapping")

    output = Path(args.output)
    output.mkdir(parents=True, exist_ok=True)
    (output / "SetupVaultAdapter.java").write_text(
        '''import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.util.Objects;

public class SetupVaultAdapter {
    private final Slot mnemonic = new Slot();

'''
        + conversion
        + '''

    private static final class Slot {
        private String value;
        void postValue(String value) { this.value = value; }
    }

    private static final class Bip39 {
        static String generateMnemonic(String entropy) { return entropy; }
    }

    private static final class Hex {
        static String toHexString(byte[] bytes) {
            StringBuilder output = new StringBuilder(bytes.length * 2);
            for (byte value : bytes) output.append(String.format("%02x", value & 0xff));
            return output.toString();
        }
    }

    private static final class HashUtil {
'''
        + sha256
        + '''
    }

    public static void main(String[] args) {
        if (args.length != 1) throw new IllegalArgumentException("expected D6 rolls");
        byte[] rolls = new byte[args[0].length()];
        for (int i = 0; i < rolls.length; ++i) {
            char face = args[0].charAt(i);
            if (face < '1' || face > '6') throw new IllegalArgumentException("invalid face");
            rolls[i] = (byte)(face - '0');
        }
        SetupVaultAdapter adapter = new SetupVaultAdapter();
        adapter.generateMnemonicFromDiceRolls(rolls);
        System.out.println(adapter.mnemonic.value);
    }
}
'''
    )
    (output / "RollingDiceAdapter.java").write_text(
        '''public class RollingDiceAdapter {
    private int currentPos;
    private boolean navigated;
    private final Activity mActivity = new Activity();

'''
        + completion
        + '''

    private void navigateToGenerateMnemonic() { navigated = true; }

    private static final class R {
        private static final class layout {
            static final int common_modal = 1;
            static final int modal_with_two_button = 2;
        }
        private static final class string {
            static final int rolling_not_enough = 1;
            static final int rolling_hint_less_than_50 = 2;
            static final int know = 3;
            static final int keep_rolling = 4;
            static final int rolling_hint_less_than_99 = 5;
            static final int confirm_rolling = 6;
        }
    }

    private static class View {
        static final int GONE = 8;
        interface OnClickListener { void onClick(View view); }
        private OnClickListener listener;
        void setOnClickListener(OnClickListener listener) { this.listener = listener; }
        void setText(Object ignored) {}
        void setVisibility(int ignored) {}
        void click() { listener.onClick(this); }
    }

    private static final class CommonModalBinding {
        final View title = new View();
        final View subTitle = new View();
        final View confirm = new View();
        final View close = new View();
    }

    private static final class ModalWithTwoButtonBinding {
        final View title = new View();
        final View subTitle = new View();
        final View left = new View();
        final View right = new View();
    }

    private static final class DataBindingUtil {
        @SuppressWarnings("unchecked")
        static <T> T inflate(Object ignored, int layout, Object root, boolean attach) {
            return (T)(layout == R.layout.common_modal
                ? new CommonModalBinding()
                : new ModalWithTwoButtonBinding());
        }
    }

    private static final class LayoutInflater {
        static Object from(Activity ignored) { return new Object(); }
    }

    private static final class Activity {
        String getString(int id, int position) { return Integer.toString(position); }
        Object getSupportFragmentManager() { return new Object(); }
    }

    private static final class ModalDialog {
        static View confirmation;
        void dismiss() {}
        void setBinding(Object binding) {
            if (binding instanceof ModalWithTwoButtonBinding) {
                confirmation = ((ModalWithTwoButtonBinding)binding).left;
            }
        }
        void show(Object manager, String tag) {}
    }

    public static void main(String[] args) {
        RollingDiceAdapter adapter = new RollingDiceAdapter();
        adapter.currentPos = Integer.parseInt(args[0]);
        ModalDialog.confirmation = null;
        adapter.onCompleteClick();
        if (adapter.navigated) {
            System.out.println("direct");
        } else if (ModalDialog.confirmation != null) {
            ModalDialog.confirmation.click();
            System.out.println(adapter.navigated ? "confirm" : "blocked");
        } else {
            System.out.println("blocked");
        }
    }
}
'''
    )


if __name__ == "__main__":
    main()
