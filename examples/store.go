// Package main implements a small, file-backed to-do list.
//
// The whole list lives in one JSON file. Every mutation rewrites that file
// atomically, so an interrupted write cannot leave a half-written list behind.
package main

import (
	"bytes"
	"encoding/json"
	"errors"
	"fmt"
	"io/fs"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"time"
)

// errNotFound is returned when no item carries the requested ID.
var errNotFound = errors.New("item not found")

// Item is one entry in the list.
type Item struct {
	ID        int        `json:"id"`
	Text      string     `json:"text"`
	Done      bool       `json:"done"`
	CreatedAt time.Time  `json:"created_at"`
	DoneAt    *time.Time `json:"done_at,omitempty"`
}

// List is the on-disk document.
type List struct {
	NextID int    `json:"next_id"`
	Items  []Item `json:"items"`
}

// Store is a handle to one list file.
type Store struct {
	path string
	list List
}

// DefaultPath returns the list location: $TODO_FILE when set, otherwise
// ~/.gotodo.json.
func DefaultPath() string {
	if p := strings.TrimSpace(os.Getenv("TODO_FILE")); p != "" {
		return p
	}
	home, err := os.UserHomeDir()
	if err != nil {
		return ".gotodo.json"
	}
	return filepath.Join(home, ".gotodo.json")
}

// Open loads the list at path. A missing or empty file yields an empty list,
// not an error, and is left on disk untouched until the first write.
func Open(path string) (*Store, error) {
	s := &Store{path: path, list: List{NextID: 1}}

	data, err := os.ReadFile(path)
	switch {
	case errors.Is(err, fs.ErrNotExist):
		return s, nil
	case err != nil:
		return nil, err
	}
	if len(bytes.TrimSpace(data)) == 0 {
		return s, nil
	}
	if err := json.Unmarshal(data, &s.list); err != nil {
		return nil, fmt.Errorf("read %s: %w", path, err)
	}
	if s.list.NextID < 1 {
		s.list.NextID = 1
	}
	for _, item := range s.list.Items {
		if item.ID >= s.list.NextID {
			s.list.NextID = item.ID + 1
		}
	}
	return s, nil
}

// Path reports the file backing this store.
func (s *Store) Path() string { return s.path }

// Add appends an item and returns it with its assigned ID.
func (s *Store) Add(text string) (Item, error) {
	text = strings.TrimSpace(text)
	if text == "" {
		return Item{}, errors.New("text is empty")
	}
	item := Item{
		ID:        s.list.NextID,
		Text:      text,
		CreatedAt: now(),
	}
	s.list.NextID++
	s.list.Items = append(s.list.Items, item)
	if err := s.save(); err != nil {
		s.list.NextID--
		s.list.Items = s.list.Items[:len(s.list.Items)-1]
		return Item{}, err
	}
	return item, nil
}

// Edit replaces the text of one item and returns the updated item. The ID,
// completion state, and timestamps are left alone.
func (s *Store) Edit(id int, text string) (Item, error) {
	text = strings.TrimSpace(text)
	if text == "" {
		return Item{}, errors.New("text is empty")
	}
	for i := range s.list.Items {
		if s.list.Items[i].ID != id {
			continue
		}
		previous := s.list.Items[i]
		updated := previous
		updated.Text = text
		s.list.Items[i] = updated
		if err := s.save(); err != nil {
			s.list.Items[i] = previous
			return Item{}, err
		}
		return updated, nil
	}
	return Item{}, fmt.Errorf("%w: id %d", errNotFound, id)
}

// Items returns the list ordered by ID. Completed items appear only when
// includeDone is true.
func (s *Store) Items(includeDone bool) []Item {
	out := make([]Item, 0, len(s.list.Items))
	for _, item := range s.list.Items {
		if item.Done && !includeDone {
			continue
		}
		out = append(out, item)
	}
	sort.Slice(out, func(i, j int) bool { return out[i].ID < out[j].ID })
	return out
}

// Complete marks one item done or not done.
func (s *Store) Complete(id int, done bool) error {
	for i := range s.list.Items {
		if s.list.Items[i].ID != id {
			continue
		}
		previous := s.list.Items[i]
		s.list.Items[i].Done = done
		if done {
			stamp := now()
			s.list.Items[i].DoneAt = &stamp
		} else {
			s.list.Items[i].DoneAt = nil
		}
		if err := s.save(); err != nil {
			s.list.Items[i] = previous
			return err
		}
		return nil
	}
	return fmt.Errorf("%w: id %d", errNotFound, id)
}

// Remove deletes one item.
func (s *Store) Remove(id int) error {
	for i, item := range s.list.Items {
		if item.ID != id {
			continue
		}
		s.list.Items = append(s.list.Items[:i], s.list.Items[i+1:]...)
		if err := s.save(); err != nil {
			// Re-insert at the same index to leave the list as it was.
			s.list.Items = append(s.list.Items, Item{})
			copy(s.list.Items[i+1:], s.list.Items[i:])
			s.list.Items[i] = item
			return err
		}
		return nil
	}
	return fmt.Errorf("%w: id %d", errNotFound, id)
}

// ClearDone deletes every completed item and reports how many it removed.
func (s *Store) ClearDone() (int, error) {
	kept := make([]Item, 0, len(s.list.Items))
	removed := 0
	for _, item := range s.list.Items {
		if item.Done {
			removed++
			continue
		}
		kept = append(kept, item)
	}
	if removed == 0 {
		return 0, nil
	}
	previous := s.list.Items
	s.list.Items = kept
	if err := s.save(); err != nil {
		s.list.Items = previous
		return 0, err
	}
	return removed, nil
}

func now() time.Time {
	return time.Now().UTC().Truncate(time.Second)
}

func (s *Store) save() error {
	sort.Slice(s.list.Items, func(i, j int) bool { return s.list.Items[i].ID < s.list.Items[j].ID })
	if s.list.Items == nil {
		s.list.Items = []Item{}
	}

	data, err := json.MarshalIndent(s.list, "", "  ")
	if err != nil {
		return err
	}
	data = append(data, '\n')

	dir := filepath.Dir(s.path)
	if dir != "" {
		if err := os.MkdirAll(dir, 0o700); err != nil {
			return err
		}
	}

	tmp, err := os.CreateTemp(dir, ".gotodo-*.tmp")
	if err != nil {
		return err
	}
	tmpName := tmp.Name()
	defer os.Remove(tmpName) // no-op after a successful rename

	if _, err := tmp.Write(data); err != nil {
		tmp.Close()
		return err
	}
	if err := tmp.Close(); err != nil {
		return err
	}
	if err := os.Chmod(tmpName, 0o600); err != nil {
		return err
	}
	return os.Rename(tmpName, s.path)
}
