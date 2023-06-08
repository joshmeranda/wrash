package args

import "fmt"

type ErrUnexpectedEOF struct {
	Cause error
}

func (e ErrUnexpectedEOF) Error() string {
	if e.Cause != nil {
		return "unexpected EOF: " + e.Cause.Error()
	}

	return "unexpected EOF"
}

type ErrInvalidEscape struct {
	Value string
}

func (e ErrInvalidEscape) Error() string {
	return "invalid escape: \\" + e.Value
}

type ErrUnexpectedToken struct {
	Expected []string
	Found    string
}

func (e ErrUnexpectedToken) Error() string {
	if len(e.Expected) == 0 {
		return fmt.Sprintf("unexpected token: %s", e.Found)
	}

	if len(e.Expected) == 1 {
		return fmt.Sprintf("unexpected token: expected %s but found %s", e.Expected[0], e.Found)
	}

	return fmt.Sprintf("unexpected token: expected one of %v but found %s", e.Expected, e.Found)
}

type ErrUnterminatedSequence struct {
	Start string
	End   string
}

func (e ErrUnterminatedSequence) Error() string {
	return fmt.Sprintf("unterminated sequence: %s ... %s", e.Start, e.End)
}

type ErrInvalidIdentifier struct {
	Identifier string
}

func (e ErrInvalidIdentifier) Error() string {
	return fmt.Sprintf("invalid identifier: '%s'", e.Identifier)
}
