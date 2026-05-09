;;; org-roam-skill-history.el --- Reading history functions -*- lexical-binding: t; -*-

;; Copyright (C) 2025

;; Keywords: outlines convenience

;;; Commentary:
;; Functions for managing reading history in quarterly files.
;; Reading history is NOT org-roam nodes - it's a consumption log.

;;; Code:

(require 'cl-lib)
(require 'org-roam)
(require 'org-roam-skill-core)
(require 'url-util)

(defcustom org-roam-skill-to-read-file nil
  "Path to the org file used by `org-roam-skill-add-to-read' for the read-later inbox.
If nil, defaults to `todo.org' in the parent directory of
`org-roam-directory' (typical setup: org-roam notes live under
`~/notes/roam/' and `todo.org' lives at `~/notes/todo.org').
The file must contain a top-level `* Inbox' heading."
  :type '(choice (const :tag "Auto: todo.org alongside org-roam-directory" nil)
                 (file :tag "Custom file"))
  :group 'org-roam)

(defun org-roam-skill--get-quarter-file ()
  "Return path to current quarter's reading history file."
  (let* ((month (string-to-number (format-time-string "%m")))
         (year (format-time-string "%Y"))
         (quarter (cond ((< month 4) "Q1")
                        ((< month 7) "Q2")
                        ((< month 10) "Q3")
                        (t "Q4"))))
    (expand-file-name (format "read_history/%s-%s.org" year quarter)
                      org-roam-directory)))

(defun org-roam-skill--ensure-quarter-file ()
  "Ensure current quarter's reading history file exists with proper header."
  (let* ((file-path (org-roam-skill--get-quarter-file))
         (dir (file-name-directory file-path))
         (month (string-to-number (format-time-string "%m")))
         (year (format-time-string "%Y"))
         (quarter (cond ((< month 4) "Q1")
                        ((< month 7) "Q2")
                        ((< month 10) "Q3")
                        (t "Q4"))))
    (unless (file-exists-p dir)
      (make-directory dir t))
    (unless (file-exists-p file-path)
      (with-temp-file file-path
        (insert (format "#+title: %s %s Reading History\n" year quarter))
        (insert "#+filetags: :reading:\n\n")))
    file-path))

;;;###autoload
(cl-defun org-roam-skill-add-reading-history
    (title url &key tags source author summary points rating)
  "Add a reading history entry to the current quarter file.

TITLE is the article title (required).
URL is the source URL (required).
TAGS is a list of tag strings for classification.
SOURCE is the website name (e.g., \"cnblogs\", \"github\").
AUTHOR is the author name or account name.
SUMMARY is a one-line summary of the content.
POINTS is a list of key points (strings).
RATING is optional 1-5 rating.

Returns the file path where entry was added."
  (let* ((file-path (org-roam-skill--ensure-quarter-file))
         (timestamp (format-time-string "%Y%m%d%H%M%S"))
         (tag-str (if tags
                      (concat " :" (mapconcat #'org-roam-skill--sanitize-tag tags ":") ":")
                    "")))
    (with-current-buffer (find-file-noselect file-path)
      (goto-char (point-max))
      (unless (bolp) (insert "\n"))
      ;; Ensure blank line before new entry (separate from previous entry)
      (unless (looking-back "\n\n" nil t)
        (insert "\n"))
      ;; Entry headline with tags
      (insert (format "* %s%s\n" title tag-str))
      ;; Properties drawer
      (insert ":PROPERTIES:\n")
      (insert (format ":URL:      %s\n" url))
      (insert (format ":READ_AT:  %s\n" timestamp))
      (when author
        (insert (format ":AUTHOR:   %s\n" author)))
      (when source
        (insert (format ":SOURCE:   %s\n" source)))
      (when rating
        (insert (format ":RATING:   %d\n" rating)))
      (insert ":END:\n\n")
      ;; Summary
      (when summary
        (insert summary)
        (insert "\n"))
      ;; Key points
      (when points
        (dolist (point points)
          (insert (format "- %s\n" point))))
      ;; Archive link
      (insert "\n")
      (insert (format "[[%s][original]] | [[https://archive.today/submit/?url=%s][archive]]\n"
                      url (url-hexify-string url)))
      (save-buffer)
      (kill-buffer))
    ;; Open archive.today submission in browser
    (browse-url (format "https://archive.today/submit/?url=%s"
                        (url-hexify-string url)))
    file-path))

;;;###autoload
(cl-defun org-roam-skill-add-to-read (title url &key summary)
  "Add a TODO item to read later under `* Inbox' in the read-later file.

TITLE is the article title (required).
URL is the link to read later (required).
SUMMARY is a brief description of what it's about.

The destination file is `org-roam-skill-to-read-file' if set; otherwise
`todo.org' in the parent directory of `org-roam-directory'.

Returns the file path where entry was added."
  (let* ((file-path (or org-roam-skill-to-read-file
                        (expand-file-name "todo.org"
                                          (file-name-directory
                                           (directory-file-name org-roam-directory)))))
         (timestamp (format-time-string "[%Y-%m-%d %a %H:%M]")))
    (with-current-buffer (find-file-noselect file-path)
      ;; Find Inbox heading
      (goto-char (point-min))
      (if (re-search-forward "^\\* Inbox" nil t)
          (progn
            (forward-line 1)
            ;; Insert new TODO
            (insert (format "** TODO %s\n" title))
            (insert ":PROPERTIES:\n")
            (insert (format ":URL:      %s\n" url))
            (insert (format ":CREATED:  %s\n" timestamp))
            (insert ":END:\n")
            (when summary
              (insert (format "%s\n" summary)))
            (insert "\n"))
        (error "No `* Inbox' heading found in %s" file-path))
      (save-buffer)
      (kill-buffer))
    file-path))

(defun org-roam-skill--get-toolkit-quarter-file ()
  "Return path to current quarter's toolkit file."
  (let* ((month (string-to-number (format-time-string "%m")))
         (year (format-time-string "%Y"))
         (quarter (cond ((< month 4) "Q1")
                        ((< month 7) "Q2")
                        ((< month 10) "Q3")
                        (t "Q4"))))
    (expand-file-name (format "toolkit/%s-%s.org" year quarter)
                      org-roam-directory)))

(defun org-roam-skill--ensure-toolkit-quarter-file ()
  "Ensure current quarter's toolkit file exists with proper header."
  (let* ((file-path (org-roam-skill--get-toolkit-quarter-file))
         (dir (file-name-directory file-path))
         (month (string-to-number (format-time-string "%m")))
         (year (format-time-string "%Y"))
         (quarter (cond ((< month 4) "Q1")
                        ((< month 7) "Q2")
                        ((< month 10) "Q3")
                        (t "Q4"))))
    (unless (file-exists-p dir)
      (make-directory dir t))
    (unless (file-exists-p file-path)
      (with-temp-file file-path
        (insert (format "#+title: %s %s Toolkit\n" year quarter))
        (insert "#+filetags: :toolkit:\n\n")))
    file-path))

;;;###autoload
(cl-defun org-roam-skill-add-toolkit-resource
    (title url &key tags category description)
  "Add a toolkit resource entry to the current quarter file.

TITLE is the resource name (required).
URL is the source URL (required).
TAGS is a list of tag strings for classification.
CATEGORY is the resource type: library, tool, service, api.
DESCRIPTION is a one-line description of the resource.

Returns the file path where entry was added."
  (let* ((file-path (org-roam-skill--ensure-toolkit-quarter-file))
         (date-str (format-time-string "%Y%m%d"))
         (tag-str (if tags
                      (concat " :" (mapconcat #'org-roam-skill--sanitize-tag tags ":") ":")
                    "")))
    (with-current-buffer (find-file-noselect file-path)
      (goto-char (point-max))
      (unless (bolp) (insert "\n"))
      ;; Ensure blank line before new entry
      (unless (looking-back "\n\n" nil t)
        (insert "\n"))
      ;; Entry headline with tags
      (insert (format "* %s%s\n" title tag-str))
      ;; Properties drawer
      (insert ":PROPERTIES:\n")
      (insert (format ":URL:      %s\n" url))
      (when category
        (insert (format ":CATEGORY: %s\n" category)))
      (insert (format ":FOUND_AT: %s\n" date-str))
      (insert ":END:\n\n")
      ;; Description
      (when description
        (insert description)
        (insert "\n"))
      (save-buffer)
      (kill-buffer))
    file-path))

(provide 'org-roam-skill-history)
;;; org-roam-skill-history.el ends here
