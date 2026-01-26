SHELL := /bin/bash
# Try to determine the artifact name. If this does not work replace it with the explicit name.
ARTIFACT := $(shell cargo pkgid |  rev | cut -d "/" -f1  | rev | cut -d "#" -f1)

# this needs to be overridden when running ssh-setup, ssh-send-key, and ssh-update-ip
TARGET := ev3

all: build deploy run

ssh-setup:
	@user=$$(echo $(TARGET) | cut -d@ -f1) && \
	host=$$(echo $(TARGET) | cut -d@ -f2) && \
	ssh-keygen -f ~/.ssh/ev3-key -N '' && \
	ssh-copy-id -i ~/.ssh/ev3-key $(TARGET) && \
	mkdir -p ~/.ssh/sockets && \
	cat >> ~/.ssh/config << EOF
	Host ev3
		HostName $$host
		User $$user
		IdentityFile ~/.ssh/ev3-key
		ControlMaster auto
		ControlPath ~/.ssh/sockets/%r@%h-%p
		ControlPersist 10m
	EOF
ssh-send-key:
	ssh-copy-id -i ~/.ssh/ev3-key $(TARGET)
ssh-update-ip:
	@host=$$(echo $(TARGET) | cut -d@ -f2) && \
	if grep -q "^Host ev3" ~/.ssh/config; then \
		sed -i '/^Host ev3/,/^$$/ s/HostName .*/HostName '"$$host"'/' ~/.ssh/config; \
	else \
		echo "Host ev3 doesn't exist! Please use make ssh-setup"; \
	fi
build:
	cargo build --release --target armv5te-unknown-linux-musleabi && \
	upx $(PWD)/target/armv5te-unknown-linux-musleabi/release/$(ARTIFACT)
deploy:
	scp $(PWD)/target/armv5te-unknown-linux-musleabi/release/$(ARTIFACT) $(TARGET):.
run:
	ssh $(TARGET) brickrun -r ./$(ARTIFACT)
