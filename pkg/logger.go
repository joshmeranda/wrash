package wrash

import (
	"context"
	"log/slog"
)

// discardHandler is a slog.Handler that discards all log entires.
type discardHandler struct{}

func (d *discardHandler) Enabled(context.Context, slog.Level) bool {
	return false
}

func (d *discardHandler) Handle(context.Context, slog.Record) error {
	return nil
}

func (d *discardHandler) WithAttrs(attrs []slog.Attr) slog.Handler {
	return d
}

func (d *discardHandler) WithGroup(name string) slog.Handler {
	return d
}
