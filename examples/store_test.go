package main

import (
	"errors"
	"io/fs"
	"os"
	"path/filepath"
	"testing"
)

func newStore(t *testing.T) *Store {
	t.Helper()
	store, err := Open(filepath.Join(t.TempDir(), "todo.json"))
	if err != nil {
		t.Fatalf("Open: %v", err)
	}
	return store
}

func addAll(t *testing.T, store *Store, texts ...string) {
	t.Helper()
	for _, text := range texts {
		if _, err := store.Add(text); err != nil {
			t.Fatalf("Add(%q): %v", text, err)
		}
	}
}

func TestOpenMissingFileIsEmpty(t *testing.T) {
	path := filepath.Join(t.TempDir(), "todo.json")

	store, err := Open(path)
	if err != nil {
		t.Fatalf("Open: %v", err)
	}
	if got := len(store.Items(true)); got != 0 {
		t.Errorf("got %d items, want 0", got)
	}
	if _, err := os.Stat(path); !errors.Is(err, fs.ErrNotExist) {
		t.Errorf("Open touched %s (stat error: %v)", path, err)
	}
}

func TestAddAssignsIncreasingIDs(t *testing.T) {
	store := newStore(t)
	addAll(t, store, "buy milk", "walk dog")

	items := store.Items(true)
	if len(items) != 2 {
		t.Fatalf("got %d items, want 2", len(items))
	}
	if items[0].ID != 1 || items[1].ID != 2 {
		t.Errorf("got ids %d, %d; want 1, 2", items[0].ID, items[1].ID)
	}
	if items[0].Text != "buy milk" {
		t.Errorf("got text %q, want %q", items[0].Text, "buy milk")
	}
	if items[0].Done {
		t.Error("new item is already done")
	}
}

func TestAddRejectsBlankText(t *testing.T) {
	store := newStore(t)

	for _, text := range []string{"", "   ", "\t\n"} {
		if _, err := store.Add(text); err == nil {
			t.Errorf("Add(%q) succeeded, want error", text)
		}
	}
	if got := len(store.Items(true)); got != 0 {
		t.Errorf("got %d items after rejected adds, want 0", got)
	}
}

func TestEditRewritesTextOnly(t *testing.T) {
	store := newStore(t)
	addAll(t, store, "buy milk", "walk dog")
	if err := store.Complete(1, true); err != nil {
		t.Fatalf("Complete: %v", err)
	}

	item, err := store.Edit(1, "  buy oat milk  ")
	if err != nil {
		t.Fatalf("Edit: %v", err)
	}
	if item.ID != 1 || item.Text != "buy oat milk" {
		t.Errorf("got %+v, want id 1 with trimmed text", item)
	}
	if !item.Done || item.DoneAt == nil {
		t.Error("Edit cleared the completion state")
	}

	items := store.Items(true)
	if items[0].Text != "buy oat milk" {
		t.Errorf("stored text: got %q, want %q", items[0].Text, "buy oat milk")
	}
	if items[0].CreatedAt.IsZero() {
		t.Error("Edit cleared created_at")
	}
	if items[0].ID != 1 || items[1].Text != "walk dog" {
		t.Errorf("Edit disturbed the other item: %+v", items)
	}
}

func TestEditRejectsBlankTextAndUnknownID(t *testing.T) {
	store := newStore(t)
	addAll(t, store, "buy milk")

	if _, err := store.Edit(1, "   "); err == nil {
		t.Error("Edit with blank text succeeded, want error")
	}
	if items := store.Items(true); items[0].Text != "buy milk" {
		t.Errorf("rejected Edit changed the text to %q", items[0].Text)
	}
	if _, err := store.Edit(99, "nope"); !errors.Is(err, errNotFound) {
		t.Errorf("unknown id: got %v, want errNotFound", err)
	}
}

func TestCompleteTogglesAndFilters(t *testing.T) {
	store := newStore(t)
	addAll(t, store, "buy milk", "walk dog")

	if err := store.Complete(2, true); err != nil {
		t.Fatalf("Complete: %v", err)
	}
	if got := len(store.Items(false)); got != 1 {
		t.Errorf("open items: got %d, want 1", got)
	}
	if got := len(store.Items(true)); got != 2 {
		t.Errorf("all items: got %d, want 2", got)
	}
	all := store.Items(true)
	if !all[1].Done || all[1].DoneAt == nil {
		t.Error("completed item is missing its done flag or timestamp")
	}
	if all[0].Done || all[0].DoneAt != nil {
		t.Error("open item was marked done")
	}

	if err := store.Complete(2, false); err != nil {
		t.Fatalf("Complete(false): %v", err)
	}
	all = store.Items(true)
	if all[1].Done || all[1].DoneAt != nil {
		t.Error("undone item still carries done state")
	}
}

func TestCompleteUnknownID(t *testing.T) {
	store := newStore(t)
	addAll(t, store, "buy milk")

	err := store.Complete(99, true)
	if !errors.Is(err, errNotFound) {
		t.Errorf("got %v, want errNotFound", err)
	}
}

func TestRemove(t *testing.T) {
	store := newStore(t)
	addAll(t, store, "a", "b", "c")

	if err := store.Remove(2); err != nil {
		t.Fatalf("Remove: %v", err)
	}
	items := store.Items(true)
	if len(items) != 2 || items[0].ID != 1 || items[1].ID != 3 {
		t.Fatalf("got %+v, want ids 1 and 3", items)
	}
	if err := store.Remove(2); !errors.Is(err, errNotFound) {
		t.Errorf("second Remove: got %v, want errNotFound", err)
	}
}

func TestClearDone(t *testing.T) {
	store := newStore(t)
	addAll(t, store, "a", "b", "c")
	if err := store.Complete(1, true); err != nil {
		t.Fatalf("Complete: %v", err)
	}
	if err := store.Complete(3, true); err != nil {
		t.Fatalf("Complete: %v", err)
	}

	removed, err := store.ClearDone()
	if err != nil {
		t.Fatalf("ClearDone: %v", err)
	}
	if removed != 2 {
		t.Errorf("got %d removed, want 2", removed)
	}
	items := store.Items(true)
	if len(items) != 1 || items[0].ID != 2 {
		t.Fatalf("got %+v, want only id 2", items)
	}

	removed, err = store.ClearDone()
	if err != nil {
		t.Fatalf("second ClearDone: %v", err)
	}
	if removed != 0 {
		t.Errorf("got %d removed, want 0", removed)
	}
}

func TestFilesPersistAcrossReopen(t *testing.T) {
	path := filepath.Join(t.TempDir(), "nested", "todo.json")

	store, err := Open(path)
	if err != nil {
		t.Fatalf("Open: %v", err)
	}
	addAll(t, store, "buy milk", "walk dog")
	if err := store.Complete(1, true); err != nil {
		t.Fatalf("Complete: %v", err)
	}
	if _, err := store.Edit(2, "walk the dog"); err != nil {
		t.Fatalf("Edit: %v", err)
	}

	reopened, err := Open(path)
	if err != nil {
		t.Fatalf("reopen: %v", err)
	}
	items := reopened.Items(true)
	if len(items) != 2 {
		t.Fatalf("got %d items, want 2", len(items))
	}
	if !items[0].Done || items[0].Text != "buy milk" {
		t.Errorf("got %+v, want done item %q", items[0], "buy milk")
	}
	if items[1].Text != "walk the dog" {
		t.Errorf("edited text did not survive the round trip: %+v", items[1])
	}
	if items[0].CreatedAt.IsZero() {
		t.Error("created_at did not survive the round trip")
	}

	item, err := reopened.Add("ship it")
	if err != nil {
		t.Fatalf("Add after reopen: %v", err)
	}
	if item.ID != 3 {
		t.Errorf("got id %d, want 3", item.ID)
	}
}

func TestItemIDValidation(t *testing.T) {
	for _, args := range [][]string{nil, {}, {"1", "2"}, {"zero"}, {"0"}, {"-3"}} {
		if _, err := itemID(args); err == nil {
			t.Errorf("itemID(%q) succeeded, want error", args)
		}
	}
	id, err := itemID([]string{"7"})
	if err != nil {
		t.Fatalf("itemID: %v", err)
	}
	if id != 7 {
		t.Errorf("got %d, want 7", id)
	}
}
