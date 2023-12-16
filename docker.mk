## detect ARCH for buildx
ARCH                                   :=$(shell uname -m)
export ARCH
ifeq ($(ARCH),x86_64)
TARGET                                 :=amd64
export TARGET
endif
ifeq ($(ARCH),arm64)
TARGET                                 :=arm64
export TARGET
endif

DOCKER=$(shell which docker)
export DOCKER
PWD=$(shell echo `pwd`)
export PWD

default:
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?##/ {printf "\033[36m%-15s\033[0m %s\n", $$1, $$2}' $(MAKEFILE_LIST)

dockerx:docker-buildx## 	docker-buildx
docker-build:## 	ocker build -f Dockerfile -t gnostr-command .
	@./gnostr-command-docker -df start
	@$(DOCKER) pull ghcr.io/gnostr-org/gnostr-command:latest
	@$(DOCKER) build -f Dockerfile -t gnostr-command .
docker-buildx:## 	docker buildx build sequence
	@./gnostr-command-docker -df start
	@$(DOCKER) run --privileged --rm tonistiigi/binfmt --install all
	@$(DOCKER) buildx ls
	@$(DOCKER) buildx create --use --name gnostr-command-buildx || true
	@$(DOCKER) buildx build -t gnostr-command --platform linux/arm64,linux/amd64 .
	@$(DOCKER) buildx build -t gnostr-command --platform linux/$(TARGET) . --load

docker-package-buildx:
	@docker build . --tag ghcr.io/gnostr-org/gnostr-command:latest
docker-package-pushx:
	@$(DOCKER) push ghcr.io/gnostr-org/gnostr-command:latest
