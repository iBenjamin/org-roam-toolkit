;;; org-roam-skill-probes.el --- Probe functions for dashboard / MCP -*- lexical-binding: t; -*-

;; Author: Ben
;; Keywords: outlines convenience

;;; Commentary:

;; Probes that produce JSON-string output for the dashboard server and
;; MCP resources. Output shape is intentionally flat and ASCII-safe.
;;
;;   org-roam-skill-probe-config       → roam directory & DB metadata
;;   org-roam-skill-probe-graph-stats  → node/edge/orphan/tag counts

;;; Code:

(require 'org-roam)
(require 'claude-skill-base)
(require 'org-roam-skill-utils)

(defun org-roam-skill--list-subdirs (dir)
  "Return immediate subdirectory names of DIR (excluding dotfiles)."
  (when (file-directory-p dir)
    (seq-filter
     (lambda (name) (not (string-prefix-p "." name)))
     (mapcar #'file-name-nondirectory
             (seq-filter #'file-directory-p
                         (directory-files dir t "^[^.].*"))))))

(defun org-roam-skill-probe-config ()
  "Return org-roam configuration as a JSON string.
Fields: dir, dbPath, dbExists, dbSize, subdirs."
  (let* ((dir (and (boundp 'org-roam-directory) org-roam-directory))
         (db (and (boundp 'org-roam-db-location) org-roam-db-location))
         (db-exists (and db (file-exists-p db)))
         (db-size (when db-exists (file-attribute-size (file-attributes db)))))
    (claude-skill-json-encode
     `(("dir"      . ,(or dir ""))
       ("dbPath"   . ,(or db ""))
       ("dbExists" . ,(if db-exists t :json-false))
       ("dbSize"   . ,(or db-size 0))
       ("subdirs"  . ,(or (org-roam-skill--list-subdirs dir) []))))))

(defun org-roam-skill-probe-graph-stats ()
  "Return org-roam graph statistics as a JSON string.
Fields: nodes, edges, orphans, tags."
  (let* ((stats   (org-roam-skill-get-graph-stats))
         (orphans (length (org-roam-skill-find-orphan-notes))))
    (claude-skill-json-encode
     `(("nodes"   . ,(plist-get stats :total-notes))
       ("edges"   . ,(plist-get stats :total-links))
       ("orphans" . ,orphans)
       ("tags"    . ,(plist-get stats :unique-tags))))))

(provide 'org-roam-skill-probes)
;;; org-roam-skill-probes.el ends here
