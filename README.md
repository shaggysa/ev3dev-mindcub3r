# ev3dev-mindcub3r

---
# Supported platforms
- Linux is natively supported. 
- Windows can be used with WSL2. 
- MacOS is untested but should be supported. 
If something doesn't work, please open an issue.  


### Windows setup (WSL)
Install wsl with PowerShell
```
wsl --install
```

To open a WSL shell, open the run dialog with `Win + R` and type in `wsl`.

Alternatively, you can launch wsl in an existing terminal by running the command `wsl`.

You can follow the below instructions in the WSL shell using your code editor of choice. 
Visual Studio Code is recommended because it has excellent [WSL integration](https://code.visualstudio.com/docs/remote/wsl#_from-vs-code). 

Other editors such as Zed may require custom setup or may need [rustup](https://rustup.rs/)
to be installed on Windows for code linting or autocompletion to work properly.

---
# Setup


### 1: Connect to the robot
If using Windows(WSL) or MacOS, use the [official Windows tutorial](https://www.ev3dev.org/docs/tutorials/connecting-to-the-internet-via-bluetooth/?tabs-0=windows-7) or the [official MacOS tutorial](https://www.ev3dev.org/docs/tutorials/using-bluetooth-tethering/).
If using native Linux, see [my tutorial](./LINUXTETHERING.md).

### 2: Install required packages
Ubuntu (WSL)
```bash
sudo apt update && sudo apt upgrade
sudo apt install git make upx curl build-essential
```

Fedora
```bash
sudo dnf install git make upx curl
sudo dnf groupinstall "Development Tools"
```

Arch
```bash
sudo pacman -S git make upx curl base-devel
```

MacOS
```bash
brew install upx gcc
```

### 3: Install rustup and add the armv5 target for the robot
You may have to close and re-open the terminal for the `rustup` command to be available after installing.
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add armv5te-unknown-linux-musleabi
```

### 4: Clone the repo
```bash
git clone https://github.com/shaggysa/ev3dev-rs-template.git
cd ev3dev-rs-template
```

### 5: Setup the robot for easy deployment
One major issue when using scp and ssh to send files and run commands is that it always prompts 
for a password and takes a several seconds to establish a connection. This command will set up an 
ssh key on your computer and robot to allow for passwordless logins and create a socket to
keep the robot connected when idle.

`make ssh-setup TARGET=robot@<ip>` where `<ip>` is the ip address in the top left corner of your robot.

For example, if the ip address was 169.254.255.255:

```bash
make ssh-setup TARGET=robot@169.254.255.255
```

### If using multiple robots
You can send the same ssh key to another robot using `make ssh-send-key TARGET=robot@<ip>`. For example:
```bash
make ssh-send-key TARGET=robot@169.254.255.255
```

### If the robot's ip address changes
If the robot's ip address changes (which can happen on each connection when using WSL), 
you can update the computer's entry with `make ssh-update-ip TARGET=robot@<new-ip>`. For example:
```bash
make ssh-update-ip TARGET=robot@169.254.255.254
```

### 6: Compile the program
A long compile time at the first compile is completely normal. Incremental compiles should be much faster.

```bash
make build
```

### 7: Deploy the program to your robot
```bash
make deploy
```

### 8: Run the program on your robot
While you can run the program directly from the robot's file browser, it is much easer to do it from the computer.
This also allows you to see any console output directly on the computer

```bash
make run
```

### Compiling, deploying, and running in one command
To compile, deploy, and run in a single command, you can simply run 

```bash
make
```

---

# Setup and Optimizations
 This project uses several "neat tricks" to optimize compile and deploy times. 
 You can expect turnaround times (running `make` to the program starting) of about
 6 seconds for incremental builds assuming you did `make ssh-setup`. 

These optimizations are automatically done when using the above instructions to compile and deploy 

## Cargo.toml
Under `[profile.release]`, we use a `opt-level = s` to  and `strip = true` to remove debug symbols. 

We are *not* using `lto = true` because it only saves about 25kb of binary size and ballons compile times. 

```
[profile.release]
strip = true
opt-level = "s"
```

---
## Makefile
### Compiling the program
To take advantage of the file size reductions in the release profile, we use `cargo build --release --target armv5te-unknown-linux-musleabi`.

Additionally, we run `upx` on the generated binary to compress it. Binaries compressed with `upx` are about half the 
size, function identically to uncompressed variants, and have practically zero compression or decompression overhead.

Using the above optimizations, we get a binary of about 320kb. At this size, `scp` transfers the binary in about 4 second. 

---
### Deploying the binary

Note that we are *not* using `rsync` to transfer the binary. It isn't preinstalled on the hub and doesn't produce 
any speed benefits compared to `scp` when using `upx` to compress the binary. Even on an incremental build, `rsync`
gives a 1.5x speedup whereas `upx` halves the binary size. 