;;; org-skill.el --- Claude Code skill for org-agenda and org-capture -*- lexical-binding: t; -*-

;; Author: Ben
;; Version: 1.0.0
;; Package-Requires: ((emacs "27.2"))
;; Keywords: outlines convenience

;;; Commentary:

;; This package provides functions for org-agenda and org-capture operations,
;; designed to work with Claude Code via emacsclient.
;;
;; Key features:
;; - Query agenda (today, week, todos)
;; - Capture todos and notes programmatically
;; - Archive completed tasks
;;
;; Usage:
;; Add to your Emacs configuration:
;;   (require 'org-skill)
;;
;; Then use emacsclient to call functions:
;;   emacsclient --eval "(org-skill-agenda-today)"

;;; Code:

(require 'org)
(require 'org-agenda)
(require 'org-capture)

;; Load all modules
(require 'org-skill-agenda)
(require 'org-skill-capture)

(provide 'org-skill)
;;; org-skill.el ends here
