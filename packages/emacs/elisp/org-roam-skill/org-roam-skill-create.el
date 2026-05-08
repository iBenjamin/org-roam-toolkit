;;; org-roam-skill-create.el --- Note creation functions -*- lexical-binding: t; -*-

;; Copyright (C) 2025

;; Author: Tahir Butt
;; Keywords: outlines convenience

;;; Commentary:
;; Functions for creating org-roam notes programmatically.

;;; Code:

(require 'cl-lib)
(require 'org-roam)
(require 'org-id)
(require 'url-util)
(require 'org-roam-skill-core)

;;;###autoload
(cl-defun org-roam-skill-create-note (title &key tags properties content content-file keep-file subdirectory source-url (open-archive :default))
  "Create a new org-roam note with TITLE, optional TAGS, PROPERTIES and CONTENT.
Automatically detect filename format and head content from capture
templates. Work with any org-roam configuration - no customization
required.

TAGS is a list of tag strings.
PROPERTIES is an alist of additional properties to add to the drawer,
e.g., \\='((\"GENERATOR\" . \"claude\") (\"MODEL\" . \"opus-4.5\")).
CONTENT can be provided as a string (small content) or via
CONTENT-FILE path (recommended for large content). If both are
provided, CONTENT-FILE takes priority.
SUBDIRECTORY is an optional subdirectory within org-roam-directory
where the note should be created (e.g., \"main\", \"reference\",
\"projects\", \"daily\").
SOURCE-URL is the original URL for reference notes. When provided,
a References section is automatically appended with the original link
and an archive.today submission link.
OPEN-ARCHIVE controls whether to open the archive.today submission URL
in browser after creating the note (requires SOURCE-URL). Defaults to
:default which auto-opens for reference notes (subdirectory=\"reference\").
Pass t to always open, nil to never open.

CONTENT FORMAT:
Content should be in `org-mode' format. For markdown conversion or
general `org-mode' formatting operations, use the orgmode skill before
calling this function. This skill focuses on org-roam-specific
operations (note creation, database sync, node linking).

TEMP FILE CLEANUP:
CONTENT-FILE is automatically deleted after processing if it appears
to be a temporary file (in /tmp/ or similar directory). To prevent
deletion, pass KEEP-FILE as t. This eliminates the need for manual
cleanup in shell scripts.

Return the file path of the created note."
  (let* ((file-name (org-roam-skill--expand-filename title))
         (effective-subdir (or subdirectory "main"))
         (target-dir (expand-file-name effective-subdir org-roam-directory))
         (file-path (expand-file-name file-name target-dir))
         (node-id (org-id-uuid))
         (head-content (org-roam-skill--get-head-content))
         ;; Read content from file if provided, otherwise use content parameter
         (actual-content (cond
                          (content-file (org-roam-skill--read-content-file content-file))
                          (content content)
                          (t nil))))

    (unwind-protect
        (progn
          ;; Create the selected note bucket on first use.
          (unless (file-directory-p target-dir)
            (make-directory target-dir t))

          ;; Create the file with proper org-roam structure
          (with-temp-file file-path
            ;; Insert PROPERTIES block with ID and custom properties
            (insert ":PROPERTIES:\n")
            (insert (format ":ID:       %s\n" node-id))
            (dolist (prop properties)
              (insert (format ":%s: %s\n" (car prop) (cdr prop))))
            (insert ":END:\n")

            ;; Insert head content if template specifies it
            (when (and head-content (not (string-empty-p head-content)))
              (let* ((expanded-head
                      ;; First expand ${title}
                      (replace-regexp-in-string "\\${title}" title head-content))
                     ;; Then expand time format specifiers
                     (expanded-head (org-roam-skill--expand-time-formats expanded-head)))
                (insert expanded-head)
                (unless (string-suffix-p "\n" expanded-head)
                  (insert "\n"))))

            ;; If head content doesn't include title, add it
            (unless (string-match-p "#\\+\\(?:title\\|TITLE\\):" (or head-content ""))
              (insert (format "#+TITLE: %s\n" title)))

            ;; Insert filetags if provided (sanitize to remove hyphens)
            (when tags
              (let ((sanitized-tags
                     (mapcar #'org-roam-skill--sanitize-tag tags)))
                (insert (format "#+FILETAGS: :%s:\n"
                                (mapconcat (lambda (tag) tag) sanitized-tags ":")))))

            ;; Add blank line after frontmatter
            (insert "\n")

            ;; Insert content if provided (user responsible for `org-mode' formatting)
            (when actual-content
              (insert actual-content)
              (unless (string-suffix-p "\n" actual-content)
                (insert "\n")))

            ;; Insert References section if source-url provided
            (when source-url
              (insert "\n* References\n\n")
              (insert (format "- %s: [[%s][original]] | [[https://archive.today/submit/?url=%s][submit archive]]\n"
                              title source-url (url-hexify-string source-url)))))

          ;; Sync database to register the new note
          (org-roam-db-sync)

          ;; Open archive.today submission in browser
          ;; Default behavior: auto-open for reference notes, otherwise respect explicit value
          (let ((should-open-archive
                 (cond
                  ((eq open-archive :default)
                   (and source-url (equal subdirectory "reference")))
                  (t open-archive))))
            (when (and source-url should-open-archive)
              (browse-url (format "https://archive.today/submit/?url=%s"
                                  (url-hexify-string source-url)))))

          ;; Return the file path
          file-path)

      ;; Cleanup: automatically delete temp file unless explicitly kept
      (when (and content-file
                 (not keep-file)
                 (file-exists-p content-file)
                 (org-roam-skill--looks-like-temp-file content-file))
        (condition-case err
            (delete-file content-file)
          (error
           (message "Warning: Could not delete temp file %s: %s"
                   content-file (error-message-string err))))))))

;;;###autoload
(defun org-roam-skill-create-note-with-content (title content &optional tags)
  "Create a new org-roam note with TITLE, CONTENT and optional TAGS.
This is an alias for org-roam-skill-create-note with different arg order.
Return the file path of the created note."
  (org-roam-skill-create-note title :content content :tags tags))

(provide 'org-roam-skill-create)
;;; org-roam-skill-create.el ends here
