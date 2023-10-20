package main

import (
	"fmt"
	"os"
	"os/exec"
	"strings"

	wrash "github.com/joshmeranda/wrash/pkg"
	"github.com/joshmeranda/wrash/pkg/args"
	"github.com/urfave/cli/v2"
)

var Version string = ""

func run(ctx *cli.Context) error {
	env := loadEnviron(nil)

	rawBase := strings.Join(ctx.Args().Slice(), " ")
	if rawBase == "" {
		return fmt.Errorf("no command provided")
	}

	base, err := args.Parse(rawBase)
	if err != nil {
		return fmt.Errorf("could not parse command args: %w", err)
	}

	expanded, err := base.Expand(func(s string) string {
		return env[s]
	})
	if err != nil {
		return fmt.Errorf("could not expaqnd args: %s", err)
	}

	if _, err := exec.LookPath(expanded[0]); err != nil {
		return fmt.Errorf("command not found: %s", base)
	}

	historyPath, err := GetHistoryFile()
	if err != nil {
		return err
	}

	entries, err := loadHistoryEntries(historyPath)
	if err != nil {
		return err
	}

	historyWriter, err := os.Create(historyPath)
	if err != nil {
		return nil
	}
	defer historyWriter.Close()

	history := wrash.NewHistory(rawBase, historyWriter, entries)

	session, err := wrash.NewSession(rawBase,
		wrash.OptionHistory(history),
		wrash.OptionEnvironment(env),
	)
	if err != nil {
		return err
	}

	session.Run()

	return nil
}

func main() {
	app := &cli.App{
		Name:        "wrash",
		Version:     Version,
		Description: "turn wrap any command line utility into an interactive shell",
		Flags:       []cli.Flag{},
		Action:      run,
		Authors: []*cli.Author{
			{
				Name:  "Josh Meranda",
				Email: "joshmeranda@gmail.com",
			},
		},
	}

	if err := app.Run(os.Args); err != nil {
		fmt.Fprintf(os.Stderr, "Error: %s", err)
	}
}
