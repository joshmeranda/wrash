package log

import (
	"context"
	"log/slog"
)

var logger *slog.Logger = nil

func SetLogger(l *slog.Logger) {
	logger = l
}

func Log(ctx context.Context, level slog.Level, msg string, args ...any) {
	if logger == nil {
		return
	}

	logger.Log(ctx, level, msg, args...)
}

func Debug(msg string, args ...any) {
	if logger == nil {
		return
	}

	logger.Debug(msg, args...)
}

func Info(msg string, args ...any) {
	if logger == nil {
		return
	}

	logger.Info(msg, args...)
}

func Warn(msg string, args ...any) {
	if logger == nil {
		return
	}

	logger.Warn(msg, args...)
}

func Error(msg string, args ...any) {
	if logger == nil {
		return
	}

	logger.Error(msg, args...)
}
