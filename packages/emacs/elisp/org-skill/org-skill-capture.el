;;; org-skill-capture.el --- Capture functions for org-skill -*- lexical-binding: t; -*-

;; Author: Ben
;; Keywords: outlines convenience

;;; Commentary:

;; Capture functions for org-skill.
;; Allows programmatic creation of TODOs and notes.

;;; Code:

(require 'org)
(require 'org-capture)

(defun org-skill-capture-todo (title &optional scheduled deadline priority)
  "Capture a TODO with TITLE.
Optional SCHEDULED and DEADLINE are date strings (e.g., \"2025-01-25\").
Optional PRIORITY is a character (A, B, or C)."
  (let* ((file (expand-file-name "~/Documents/org/todo.org"))
         (priority-str (when priority (format "[#%c] " priority)))
         (props (format ":PROPERTIES:\n:CREATED: %s\n:END:\n"
                        (format-time-string "[%Y-%m-%d %a %H:%M]")))
         (scheduling ""))
    ;; Build scheduling line
    (when scheduled
      (setq scheduling (concat scheduling
                                (format "SCHEDULED: <%s>\n" scheduled))))
    (when deadline
      (setq scheduling (concat scheduling
                                (format "DEADLINE: <%s>\n" deadline))))
    ;; Insert the entry
    (with-current-buffer (find-file-noselect file)
      (goto-char (point-min))
      (if (re-search-forward "^\\* Inbox" nil t)
          (progn
            (org-end-of-subtree)
            (insert (format "\n** TODO %s%s\n%s%s"
                            (or priority-str "") title scheduling props)))
        ;; No Inbox found, create at end
        (goto-char (point-max))
        (insert (format "\n* Inbox\n** TODO %s%s\n%s%s"
                        (or priority-str "") title scheduling props)))
      (save-buffer))
    (format "Created TODO: %s" title)))

(defun org-skill-capture-note (title &optional content)
  "Capture a note with TITLE and optional CONTENT."
  (let* ((file (expand-file-name "~/Documents/org/todo.org"))
         (props (format ":PROPERTIES:\n:CREATED: %s\n:END:\n"
                        (format-time-string "[%Y-%m-%d %a %H:%M]")))
         (body (or content "")))
    (with-current-buffer (find-file-noselect file)
      (goto-char (point-min))
      (if (re-search-forward "^\\* Notes" nil t)
          (progn
            (org-end-of-subtree)
            (insert (format "\n** %s\n%s%s\n" title props body)))
        ;; No Notes found, create at end
        (goto-char (point-max))
        (insert (format "\n* Notes\n** %s\n%s%s\n" title props body)))
      (save-buffer))
    (format "Created note: %s" title)))

(defun org-skill-complete-todo (heading)
  "Mark TODO with HEADING as DONE."
  (let ((found nil))
    (dolist (file org-agenda-files)
      (when (and (file-exists-p file) (not found))
        (with-current-buffer (find-file-noselect file)
          (org-map-entries
           (lambda ()
             (when (string-match-p (regexp-quote heading)
                                   (org-get-heading t t t t))
               (org-todo "DONE")
               (setq found t)
               (save-buffer)))
           "TODO=\"TODO\"" 'file))))
    (if found
        (format "Completed: %s" heading)
      (format "TODO not found: %s" heading))))

(defun org-skill-archive-done ()
  "Archive all DONE items in agenda files."
  (let ((count 0))
    (dolist (file org-agenda-files)
      (when (file-exists-p file)
        (with-current-buffer (find-file-noselect file)
          (org-map-entries
           (lambda ()
             (org-archive-subtree)
             (setq count (1+ count)))
           "TODO=\"DONE\"" 'file)
          (save-buffer))))
    (format "Archived %d items" count)))

(defun org-skill-set-priority (heading priority)
  "Set PRIORITY (A/B/C) for TODO with HEADING."
  (let ((found nil))
    (dolist (file org-agenda-files)
      (when (and (file-exists-p file) (not found))
        (with-current-buffer (find-file-noselect file)
          (org-map-entries
           (lambda ()
             (when (string-match-p (regexp-quote heading)
                                   (org-get-heading t t t t))
               (org-priority (string-to-char priority))
               (setq found t)
               (save-buffer)))
           nil 'file))))
    (if found
        (format "Set priority [#%s] for: %s" priority heading)
      (format "Entry not found: %s" heading))))

(defun org-skill-schedule-todo (heading date)
  "Schedule TODO with HEADING for DATE (e.g., \"2025-01-25\")."
  (let ((found nil))
    (dolist (file org-agenda-files)
      (when (and (file-exists-p file) (not found))
        (with-current-buffer (find-file-noselect file)
          (org-map-entries
           (lambda ()
             (when (string-match-p (regexp-quote heading)
                                   (org-get-heading t t t t))
               (org-schedule nil date)
               (setq found t)
               (save-buffer)))
           nil 'file))))
    (if found
        (format "Scheduled for %s: %s" date heading)
      (format "Entry not found: %s" heading))))

(provide 'org-skill-capture)
;;; org-skill-capture.el ends here
