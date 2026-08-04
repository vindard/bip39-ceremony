;; Authenticate the Guix history from its canonical channel introduction and
;; select the exact revision reviewed for this release-build prototype.
(list
 (channel
  (name 'guix)
  (url "https://git.guix.gnu.org/guix.git")
  (commit "86813d5779253bb50002d79ab791eeda5a8b4729")
  (introduction
   (make-channel-introduction
    "9edb3f66fd807b096b48283debdcddccfea34bad"
    (openpgp-fingerprint
     "BBB0 2DDF 2CEA F6A8 0D1D E643 A2A0 6DF2 A33A 54FA")))))
