# Executes the pinned upstream reading directly. Only `entropy.ex` is loaded,
# which depends on nothing outside Elixir's standard library, so the number this
# prints is the upstream function's own output rather than a restatement of it.

[source, rolls] = System.argv()

Code.require_file(Path.join(source, "lib/key/hd/entropy.ex"))

case BitcoinLib.Key.HD.Entropy.from_dice_rolls(rolls) do
  {:ok, value} -> IO.puts("ok\t#{Integer.to_string(value)}")
  {:error, message} -> IO.puts("error\t#{message}")
end
