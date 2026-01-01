package log

import (
	"bytes"
	"context"
	"fmt"
	"log/slog"
	"os"
	"slices"
	"strconv"
	"time"
)

const (
	AttrKeyStacktrace = "stacktrace"
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

type Options struct {
	Level slog.Leveler
}

type fileHandler struct {
	f *os.File

	level slog.Leveler
	attrs []slog.Attr
}

func NewFileHandler(path string, opts *Options) (slog.Handler, error) {
	out, err := os.OpenFile(path, os.O_WRONLY|os.O_CREATE|os.O_APPEND, 0o644)
	if err != nil {
		return nil, fmt.Errorf("failed to open log file: %w", err)
	}

	return &fileHandler{
		f: out,

		level: opts.Level,
		attrs: make([]slog.Attr, 0),
	}, nil
}

func (f *fileHandler) Enabled(_ context.Context, l slog.Level) bool {
	return l >= f.level.Level()
}

func (f *fileHandler) Handle(_ context.Context, r slog.Record) error {
	buf := bytes.NewBuffer(nil)

	var stackTrace string

	buf.WriteString(r.Time.Format(time.RFC3339) + " ")
	buf.WriteString(r.Level.String())

	r.AddAttrs(f.attrs...)
	r.AddAttrs(slog.Attr{
		Key:   "msg",
		Value: slog.StringValue(r.Message),
	})

	r.Attrs(func(attr slog.Attr) bool {
		if attr.Key == AttrKeyStacktrace {
			stackTrace = attr.Value.String()
			return true
		}

		if isQuoteable(attr.Value) {
			buf.WriteString(fmt.Sprintf(" %s=%s", attr.Key, strconv.Quote(attr.Value.String())))
		} else {
			buf.WriteString(fmt.Sprintf(" %s=%s", attr.Key, attr.Value))
		}

		return true
	})

	if stackTrace != "" {
		buf.WriteByte('\n')
		buf.WriteString(string(stackTrace))
	}

	buf.WriteByte('\n')

	if _, err := f.f.Write(buf.Bytes()); err != nil {
		return err
	}

	if err := f.f.Sync(); err != nil {
		return err
	}

	return nil
}

func (f *fileHandler) WithAttrs(attrs []slog.Attr) slog.Handler {
	return &fileHandler{
		f: f.f,

		level: f.level,
		attrs: append(slices.Clone(f.attrs), attrs...),
	}
}

// WithGroup does not hing but return a copy of the given handler. Changes to the returned handler will not affect the original.
func (f *fileHandler) WithGroup(group string) slog.Handler {
	return &fileHandler{
		f: f.f,

		level: f.level,
		attrs: slices.Clone(f.attrs),
	}
}

func isQuoteable(v slog.Value) bool {
	switch v.Kind() {
	case slog.KindString, slog.KindAny, slog.KindLogValuer:
		return true
	default:
		return false
	}
}
