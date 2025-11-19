package main

import (
	"os"
	"path"
)

const (
	EnvHistoryFile = "WRASH_HISTORY_FILE"
	EnvConfigDir   = "WRASH_CONFIG_DIR"
)

func GetHistoryFile() (string, error) {
	if path := os.Getenv(EnvHistoryFile); path != "" {
		return path, nil
	}

	dir, err := os.UserHomeDir()
	if err != nil {
		return "", err
	}

	return path.Join(dir, ".wrash_history.yaml"), nil
}

func GetConfigDir() (string, error) {
	if path := os.Getenv(EnvConfigDir); path != "" {
		return path, nil
	}

	dir, err := os.UserConfigDir()
	if err != nil {
		return "", err
	}

	return dir, nil
}
