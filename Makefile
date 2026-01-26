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
	{ \
        echo "Host ev3"; \
        echo "    HostName $$host"; \
        echo "    User $$user"; \
        echo "    IdentityFile ~/.ssh/ev3-key"; \
        echo "    ControlMaster auto"; \
    	echo "    ControlPath ~/.ssh/sockets/%r@%h-%p"; \
    	echo "    ControlPersist 10m"; \
    	} >> ~/.ssh/config && \
    	echo "SSH setup complete! " && \
    	echo "The host is $$host and the user is $$user." && \
    	echo "You can now deploy and run with the makefile for ssh into the robot with 'ssh ev3'" && \
    	echo "If the robot's ip address changes, you can update your computer's entry with make ssh-update-ip TARGET=robot@<new-ip>"

ssh-send-key:
	@ssh-copy-id -i ~/.ssh/ev3-key $(TARGET) && \
	echo "Sent the key to $(TARGET)! You should now be able to login without a password." && \
	echo "If the new robot's ip is different, you can update your computer's entry with make ssh-update-ip TARGET=robot@<new-ip>"

ssh-update-ip:
	@host=$$(echo $(TARGET) | cut -d@ -f2) && \
	if grep -q "^Host ev3" ~/.ssh/config; then \
		sed -i '/^Host ev3/,/^$$/ s/HostName .*/HostName '"$$host"'/' ~/.ssh/config && \
		echo "Set ip address for ev3 to $$host"; \
	else \
		echo "Host ev3 doesn't exist! Please use make ssh-setup TARGET=robot@<ip>"; \
	fi && \

build:
	@cargo build --release --target armv5te-unknown-linux-musleabi && \
	upx -qq $(PWD)/target/armv5te-unknown-linux-musleabi/release/$(ARTIFACT) && \
	echo "Build complete!" && \
	echo ""

deploy:
	@scp $(PWD)/target/armv5te-unknown-linux-musleabi/release/$(ARTIFACT) $(TARGET):. && \
	echo "Deploy complete!" && \
	echo ""
run:
	@ssh $(TARGET) brickrun -r ./$(ARTIFACT)