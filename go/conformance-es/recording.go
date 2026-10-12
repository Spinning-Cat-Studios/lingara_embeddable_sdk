package main

import (
	"bytes"
	"context"
	"log/slog"
	"strings"
	"sync"
	"sync/atomic"

	lingaraembed "github.com/Spinning-Cat-Studios/lingara_embeddable_sdk/go"
)

// recording is the case's MemoryDirectory with its PlayerRefFor calls
// counted: a lookup changes no contents, so only a count shows a forged
// delivery never reached the store.
type recording struct {
	*lingaraembed.MemoryDirectory
	lookups atomic.Int64
}

func seeded(seed map[string]string) *recording {
	d := &recording{MemoryDirectory: lingaraembed.NewMemoryDirectory()}
	for subject, playerRef := range seed {
		_ = d.Remember(context.Background(), playerRef, subject)
	}
	return d
}

func (d *recording) PlayerRefFor(ctx context.Context, subject string) (string, bool, error) {
	d.lookups.Add(1)
	return d.MemoryDirectory.PlayerRefFor(ctx, subject)
}

// sink holds every line the routers' onError and log/slog's default logger
// wrote, for ES5's render sweep.
type sink struct {
	mu    sync.Mutex
	lines []string
}

func (s *sink) push(line string) {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.lines = append(s.lines, line)
}

func (s *sink) take() []string {
	s.mu.Lock()
	defer s.mu.Unlock()
	lines := s.lines
	s.lines = nil
	return lines
}

// Write takes slog's text lines.
func (s *sink) Write(p []byte) (int, error) {
	for _, line := range strings.Split(string(bytes.TrimRight(p, "\n")), "\n") {
		s.push(line)
	}
	return len(p), nil
}

// captureLogs points log/slog's default logger, the router's default
// onError facility, at the sink.
func captureLogs(s *sink) {
	slog.SetDefault(slog.New(slog.NewTextHandler(s, &slog.HandlerOptions{Level: slog.LevelDebug})))
}
