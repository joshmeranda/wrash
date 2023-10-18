package main

import (
	"fmt"
	"os"
	"path"
)

const (
	EnvHistoryFile = "WRASH_HISTORY_FILE"
)

func GetHistoryFile() (string, error) {
	if path := os.Getenv(EnvHistoryFile); path != "" {
		return path, nil
	}

	dir, err := os.UserHomeDir()
	if err != nil {
		return "", fmt.Errorf("could not determine user home directory: %w", err)
	}

	return path.Join(dir, ".wrash_history.yaml"), nil
}
