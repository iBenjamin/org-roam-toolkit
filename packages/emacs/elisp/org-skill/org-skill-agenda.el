;;; org-skill-agenda.el --- Agenda functions for org-skill -*- lexical-binding: t; -*-

;; Author: Ben
;; Keywords: outlines convenience

;;; Commentary:

;; Agenda query functions for org-skill.
;; Returns data in JSON format for easy parsing.

;;; Code:

(require 'org)
(require 'org-agenda)
(require 'json)

(defun org-skill--entry-to-plist (marker)
  "Convert org entry at MARKER to a plist."
  (when (marker-buffer marker)
    (with-current-buffer (marker-buffer marker)
      (goto-char marker)
      (let* ((heading (org-get-heading t t t t))
             (todo-state (org-get-todo-state))
             (priority (org-get-priority (org-get-heading)))
             (tags (org-get-tags))
             (scheduled (org-entry-get nil "SCHEDULED"))
             (deadline (org-entry-get nil "DEADLINE"))
             (file (buffer-file-name)))
        (list :heading heading
              :todo todo-state
              :priority priority
              :tags tags
              :scheduled scheduled
              :deadline deadline
              :file file)))))

(defun org-skill--collect-agenda-entries (span)
  "Collect agenda entries for SPAN days."
  (let ((entries '())
        (org-agenda-span span)
        (org-agenda-start-on-weekday nil))
    (save-window-excursion
      (org-agenda-list nil nil span)
      (goto-char (point-min))
      (while (not (eobp))
        (let ((marker (get-text-property (point) 'org-marker)))
          (when marker
            (let ((entry (org-skill--entry-to-plist marker)))
              (when entry
                (push entry entries)))))
        (forward-line 1)))
    (nreverse entries)))

(defun org-skill-agenda-today ()
  "Return today's agenda entries as JSON."
  (let ((entries (org-skill--collect-agenda-entries 1)))
    (json-encode entries)))

(defun org-skill-agenda-week ()
  "Return this week's agenda entries as JSON."
  (let ((entries (org-skill--collect-agenda-entries 7)))
    (json-encode entries)))

(defun org-skill-agenda-todos ()
  "Return all TODO items as JSON."
  (let ((entries '()))
    (dolist (file org-agenda-files)
      (when (file-exists-p file)
        (with-current-buffer (find-file-noselect file)
          (org-map-entries
           (lambda ()
             (let* ((heading (org-get-heading t t t t))
                    (todo-state (org-get-todo-state))
                    (priority (org-get-priority (org-get-heading)))
                    (tags (org-get-tags))
                    (scheduled (org-entry-get nil "SCHEDULED"))
                    (deadline (org-entry-get nil "DEADLINE"))
                    (created (org-entry-get nil "CREATED")))
               (when todo-state
                 (push (list :heading heading
                            :todo todo-state
                            :priority priority
                            :tags tags
                            :scheduled scheduled
                            :deadline deadline
                            :created created
                            :file file)
                       entries))))
           nil 'file))))
    (json-encode (nreverse entries))))

(defun org-skill-agenda-search (query)
  "Search agenda files for QUERY and return matching entries as JSON."
  (let ((entries '()))
    (dolist (file org-agenda-files)
      (when (file-exists-p file)
        (with-current-buffer (find-file-noselect file)
          (org-map-entries
           (lambda ()
             (let ((heading (org-get-heading t t t t)))
               (when (string-match-p (regexp-quote query) heading)
                 (push (list :heading heading
                            :todo (org-get-todo-state)
                            :tags (org-get-tags)
                            :file file)
                       entries))))
           nil 'file))))
    (json-encode (nreverse entries))))

(provide 'org-skill-agenda)
;;; org-skill-agenda.el ends here
