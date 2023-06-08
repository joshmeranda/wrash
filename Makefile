GO_BUILD=go build -race
GO_FMT=go fmt
GO_TEST=go test
GO_GENERATE=go generate

ifdef VERBOSE
        GO_BUILD += -v -x
        GO_TEST += -test.v
		GO_GENERATE += -v -x

        RM += --verbose
endif

TAG=$(shell git tag --contains HEAD)

ifeq ($(TAG),)
$(info no tag found for HEAD, generating... )
TAG="$(shell git tag --sort version:refname --list | tail --lines 1)-$(shell git rev-parse HEAD)"
$(info using tag ${TAG})
endif

.PHONY: help

help:
	@echo "Available targets and values:"
	@echo "Targets:"
	@echo "  wrash 	     build wrash binary"
	@echo "  build       build all wrash artifacts"
	@echo "  clean       remove all wrash artifacts"
	@echo "  test        run tests"
	@echo ""
	@echo "Values:"
	@echo "  VERBOSE     run recipes with more verbose output"

# # # # # # # # # # # # # # # # # # # #
# Build recipes                       #
# # # # # # # # # # # # # # # # # # # #

.PHONY: build wrash

SOURCES=$(shell find . -name '*.go')

build: wrash

wrash: bin/wrash

bin/wrash: ${SOURCES}
	${GO_BUILD} -ldflags "-X wrash/pkg/cmd.Version=${TAG}" -o $@ ./pkg/cmd/wrash

# # # # # # # # # # # # # # # # # # # #
# Test recipes                        #
# # # # # # # # # # # # # # # # # # # #

TEST_PKGS=./pkg 

test:
	${GO_TEST} ${TEST_PKGS}

# # # # # # # # # # # # # # # # # # # #
# Clean recipes                       #
# # # # # # # # # # # # # # # # # # # #

clean:
	${RM} --recursive bin