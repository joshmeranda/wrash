package wrash

import (
	"bytes"
	"context"
	"fmt"
	"io"
	"log/slog"
	"os"
	"strconv"
	"time"
)

const (
	attrKeyStacktrace = "stacktrace"
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

type fileHandler struct {
	out io.Writer

	level  slog.Leveler
	attrs  []slog.Attr
	groups []string
}

func NewFileHandler(path string, opts *slog.HandlerOptions) (slog.Handler, error) {
	out, err := os.OpenFile(path, os.O_WRONLY|os.O_CREATE|os.O_APPEND, 0o644)
	if err != nil {
		return nil, fmt.Errorf("failed to open log file: %w", err)
	}

	return &fileHandler{
		out: out,

		level:  opts.Level,
		attrs:  make([]slog.Attr, 0),
		groups: make([]string, 0),
	}, nil
}

func (f *fileHandler) Enabled(_ context.Context, l slog.Level) bool {
	return l > f.level.Level()
}

// Handle implements slog.Handler.
func (f *fileHandler) Handle(_ context.Context, r slog.Record) error {
	buf := bytes.NewBuffer(nil)

	var stackTrace string

	buf.WriteString(r.Time.Format(time.RFC3339) + " ")
	buf.WriteString(r.Level.String())

	r.AddAttrs(slog.Attr{
		Key:   "msg",
		Value: slog.StringValue(r.Message),
	})

	r.Attrs(func(attr slog.Attr) bool {
		if attr.Key == attrKeyStacktrace {
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

	if _, err := f.out.Write(buf.Bytes()); err != nil {
		return err
	}

	return nil
}

func (f *fileHandler) WithAttrs(attrs []slog.Attr) slog.Handler {
	return &fileHandler{
		out: f.out,

		level:  f.level,
		attrs:  append(f.attrs, attrs...),
		groups: f.groups,
	}
}

func (f *fileHandler) WithGroup(group string) slog.Handler {
	return &fileHandler{
		out: f.out,

		level:  f.level,
		attrs:  f.attrs,
		groups: append(f.groups, group),
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
