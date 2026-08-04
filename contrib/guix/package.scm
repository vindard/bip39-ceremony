;;; Reproducible-build prototype for bip39-ceremony.

(use-modules (gnu packages rust)
             (guix build-system cargo)
             (guix git-download)
             (guix gexp)
             (guix licenses)
             (guix packages))

(define source-root
  (canonicalize-path
   (string-append (dirname (current-filename)) "/../..")))

(define (locked-crate name version hash)
  (crate-source name version hash))

;; Keep this list in exact correspondence with Cargo.lock.  Cargo's registry
;; checksums are SHA-256; Guix's base32 values encode those same digests.
(define rust-arrayvec-0.7.8
  (locked-crate "arrayvec" "0.7.8"
                "0mmd8lrijbvg1qp4c5zis5dq41a3mjv2rb6bxkyj9kwaw2k6gyyk"))
(define rust-bip39-2.2.2
  (locked-crate "bip39" "2.2.2"
                "1g6ms446z6f4dza994667vj5irgmzih1x4k3jcijjwi2k0fd7nwh"))
(define rust-bitcoin-hashes-0.14.0
  (locked-crate "bitcoin_hashes" "0.14.0"
                "05jdirz6p2q1fbg65c1xfhf8bsx7snpzm9i1g8a7w95h1lyw065v"))
(define rust-hex-conservative-0.2.2
  (locked-crate "hex-conservative" "0.2.2"
                "17qba5mg59b15gld8jz7xywzi6vj8ycipr041k26fqk0mhc6v87x"))
(define rust-libc-0.2.189
  (locked-crate "libc" "0.2.189"
                "1whjfs375vlng2q6yrbzs73cvp5lm3w1n2gfqajb2vgf7zg3xbry"))
(define rust-numtoa-0.2.4
  (locked-crate "numtoa" "0.2.4"
                "03yhkhjb3d1zx22m3pgcbpk8baj0zzvaxqc25c584sdq77jw98ka"))
(define rust-proc-macro2-1.0.107
  (locked-crate "proc-macro2" "1.0.107"
                "1nb6ly8kp65f724kj73ippc7lvydss24sm2vagk6qpklpg4pwplq"))
(define rust-quote-1.0.47
  (locked-crate "quote" "1.0.47"
                "00ch0yyzvv6s671ik0kcsbw8nigdaj2g3fr61kcahwx48aqlvgqz"))
(define rust-syn-2.0.119
  (locked-crate "syn" "2.0.119"
                "15vjy620l91a3q4n4f4gzhnflmdr6pnm38v2m6cpk86i8av32a47"))
(define rust-termion-4.0.6
  (locked-crate "termion" "4.0.6"
                "1jsy8zakr7gjy4wddb1m1hrsfkgg2wjxh121y81gbw08mslkhhgl"))
(define rust-unicode-ident-1.0.24
  (locked-crate "unicode-ident" "1.0.24"
                "0xfs8y1g7syl2iykji8zk5hgfi5jw819f5zsrbaxmlzwsly33r76"))
(define rust-zeroize-1.9.0
  (locked-crate "zeroize" "1.9.0"
                "0kpnij2v1ig6g2mhc0bnci0lrdfdhiq40afbc0fahajqc9jiag71"))
(define rust-zeroize-derive-1.5.0
  (locked-crate "zeroize_derive" "1.5.0"
                "0a7kq8srk81pn23xqn7c9jw1jpnfy41ffn802x1zrqqgpdf6al1w"))

(package
 (name "bip39-ceremony")
 (version "0.1.0")
 (source
  (local-file source-root "bip39-ceremony-checkout"
              #:recursive? #t
              #:select? (git-predicate source-root)))
 (build-system cargo-build-system)
 (arguments
  (list #:rust rust-1.94
        #:install-source? #f
        #:cargo-build-flags #~(list "--release" "--frozen"
                                    "--package" "bip39-ceremony-tui")
        #:cargo-test-flags #~(list "--workspace" "--all-targets"
                                  "--all-features" "--frozen")))
 (inputs
  (list rust-arrayvec-0.7.8
        rust-bip39-2.2.2
        rust-bitcoin-hashes-0.14.0
        rust-hex-conservative-0.2.2
        rust-libc-0.2.189
        rust-numtoa-0.2.4
        rust-proc-macro2-1.0.107
        rust-quote-1.0.47
        rust-syn-2.0.119
        rust-termion-4.0.6
        rust-unicode-ident-1.0.24
        rust-zeroize-1.9.0
        rust-zeroize-derive-1.5.0))
 (home-page "https://github.com/vindard/bip39-ceremony")
 (synopsis "Inspect physical-entropy conversion to BIP-39 mnemonics")
 (description
  "BIP-39 Ceremony is an offline terminal application that makes conversion of
physical dice and coin observations into English BIP-39 mnemonics inspectable.")
 (license mit))
