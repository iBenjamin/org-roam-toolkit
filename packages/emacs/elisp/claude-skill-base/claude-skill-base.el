;;; claude-skill-base.el --- Shared utilities for claude-skill elisp packages -*- lexical-binding: t; -*-

;; Author: Ben
;; Version: 0.1.0
;; Package-Requires: ((emacs "27.2"))
;; Keywords: outlines convenience

;;; Commentary:

;; Shared helpers used by org-skill, org-roam-skill, and any future
;; claude-skill elisp package living under
;; <repo>/packages/emacs/elisp/<pkg>/.
;;
;; Conventions:
;; - All claude-skill packages produce JSON-as-string when their output
;;   crosses the emacsclient boundary, using `claude-skill-json-encode'.
;; - Errors should be turned into JSON via `claude-skill-error-result'.

;;; Code:

(require 'json)

(defun claude-skill-json-encode (obj)
  "Encode OBJ as a JSON string suitable for emacsclient output.
Uses arrays for lists, objects for alists, with a stable key order."
  (let ((json-encoding-pretty-print nil)
        (json-encoding-default-indentation "")
        (json-encoding-object-sort-predicate #'string<))
    (json-encode obj)))

(defun claude-skill-ok-result (data)
  "Wrap DATA in a JSON success envelope: {\"ok\": true, \"data\": ...}."
  (claude-skill-json-encode `(("ok" . t) ("data" . ,data))))

(defun claude-skill-error-result (msg)
  "Wrap MSG in a JSON error envelope: {\"ok\": false, \"error\": MSG}."
  (claude-skill-json-encode `(("ok" . :json-false) ("error" . ,msg))))

(defmacro claude-skill-with-error-handling (&rest body)
  "Run BODY, returning a JSON envelope on success or error."
  (declare (indent 0))
  `(condition-case err
       (claude-skill-ok-result (progn ,@body))
     (error (claude-skill-error-result (error-message-string err)))))

(defun claude-skill-sanitize-tag (tag)
  "Sanitize TAG for use as an org tag (alphanumerics + underscore)."
  (replace-regexp-in-string "[^[:alnum:]_]" "_" (downcase tag)))

;; --- Probes ---------------------------------------------------------------

(defconst claude-skill-known-features
  '(claude-skill-base org-skill org-roam-skill)
  "Features tracked by `claude-skill-probe-daemon'.")

(defun claude-skill-probe-daemon ()
  "Return daemon health as a JSON string.
Fields: pid, uptimeSeconds, loadedFeatures."
  (claude-skill-json-encode
   `(("pid"            . ,(emacs-pid))
     ("uptimeSeconds"  . ,(float-time (time-since before-init-time)))
     ("loadedFeatures" . ,(mapcar #'symbol-name
                                  (seq-filter #'featurep
                                              claude-skill-known-features))))))

(provide 'claude-skill-base)
;;; claude-skill-base.el ends here
