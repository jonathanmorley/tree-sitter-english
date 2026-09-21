# english-pos-train

Offline trainer for the `english-pos` perceptron. Not published.

Fetches nothing: point it at a Universal Dependencies checkout
(UD English-EWT, CC BY-SA — keep the data out of the repo):

```sh
cargo run --release -p english-pos-train -- \
  --corpus /tmp/ud --iters 10 --min-count 2
```

Trains on `en_ewt-ud-train.conllu`, reports dev/test accuracy, and
writes `../english-pos/weights/upos.json`. Re-evaluate committed
weights any time with `--eval-only test` (or `dev`).

Notes:

- Plain (unaveraged) perceptron, deliberately: on fast-converging
  data, Collins averaging shrinks settled weights into noise while
  rarely-updated rare patterns dominate decoding (measured 33% vs
  88% dev on a 300-sentence pilot). Final-iteration weights win here.
- Feature templates live in `english-pos` and are shared verbatim
  with inference; changing them invalidates the committed weights.
- Tokenization follows UD (contractions split); see the `english-pos`
  README for the mismatch with `english`-crate words.
