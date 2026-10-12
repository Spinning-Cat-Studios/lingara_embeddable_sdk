package lingaraembed

import (
	"context"
	"fmt"
	"log/slog"
	"sync"
)

// PlayerDirectory is your store of player_ref ↔ subject (ADR 1.10.26af D5):
// the one mapping a kit cannot derive. A subject is a pairwise HMAC computed
// by Lingara, so the mint's answer is the only moment a player_ref and its
// subject are in one place. Implement it over your own database, beside your
// player rows, unique on subject only: a player who links an account owns a
// second subject beside the guest's.
type PlayerDirectory interface {
	// Remember upserts the row keyed on subject. A guest's subject is stable
	// across mints, so remembering it again changes nothing.
	Remember(ctx context.Context, playerRef, subject string) error
	// PlayerRefFor returns the player a subject belongs to, and false for a
	// subject this store never saw (your own subject, a deleted player).
	PlayerRefFor(ctx context.Context, subject string) (string, bool, error)
	// Forget removes every row for the player. It is idempotent.
	Forget(ctx context.Context, playerRef string) error
}

// MemoryDirectory is a process-local PlayerDirectory, for tests and
// single-process demos. It is lost on restart and not shared between
// instances: production code implements PlayerDirectory over its own
// database. It is safe for concurrent use.
type MemoryDirectory struct {
	mu   sync.Mutex
	rows map[string]string // subject → player_ref
}

// NewMemoryDirectory returns an empty MemoryDirectory.
func NewMemoryDirectory() *MemoryDirectory {
	return &MemoryDirectory{rows: map[string]string{}}
}

// Remember upserts subject → playerRef.
func (d *MemoryDirectory) Remember(_ context.Context, playerRef, subject string) error {
	d.mu.Lock()
	defer d.mu.Unlock()
	d.rows[subject] = playerRef
	return nil
}

// PlayerRefFor returns subject's player, if any.
func (d *MemoryDirectory) PlayerRefFor(_ context.Context, subject string) (string, bool, error) {
	d.mu.Lock()
	defer d.mu.Unlock()
	playerRef, ok := d.rows[subject]
	return playerRef, ok, nil
}

// Forget removes every subject playerRef owns.
func (d *MemoryDirectory) Forget(_ context.Context, playerRef string) error {
	d.mu.Lock()
	defer d.mu.Unlock()
	for subject, owner := range d.rows {
		if owner == playerRef {
			delete(d.rows, subject)
		}
	}
	return nil
}

// Snapshot returns a copy of every row, subject → player_ref.
func (d *MemoryDirectory) Snapshot() map[string]string {
	d.mu.Lock()
	defer d.mu.Unlock()
	rows := make(map[string]string, len(d.rows))
	for subject, playerRef := range d.rows {
		rows[subject] = playerRef
	}
	return rows
}

func (d *MemoryDirectory) count() int {
	d.mu.Lock()
	defer d.mu.Unlock()
	return len(d.rows)
}

// Format renders the row count only.
func (d *MemoryDirectory) Format(f fmt.State, _ rune) {
	_, _ = fmt.Fprintf(f, "lingaraembed.MemoryDirectory(%d rows)", d.count())
}

// LogValue is the row count only.
func (d *MemoryDirectory) LogValue() slog.Value {
	return slog.GroupValue(slog.Int("rows", d.count()))
}

// DirectoryError is a failure of your PlayerDirectory, as the kit passes it
// on. Unwrap returns your store's error unchanged.
type DirectoryError struct {
	Err error
}

func (e *DirectoryError) Error() string {
	return "lingaraembed: the player directory failed: " + e.Err.Error()
}

// Unwrap returns the store's own error.
func (e *DirectoryError) Unwrap() error { return e.Err }
