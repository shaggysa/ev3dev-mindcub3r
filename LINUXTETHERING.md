# Connecting to the hub on Linux
### If you are using WSL, see the [official guide](https://www.ev3dev.org/docs/tutorials/connecting-to-the-internet-via-bluetooth/) for windows.

---

### 1: On the EV3 brick, navigate to "Wireless and Networks"

![EV3 Wireless and Networks page](./images/wireless-and-networks.jpg)

### 2: In the "Wireless and Networks" menu, navigate to "Tethering"

![EV3 Tethering page](./images/tethering.jpg)

### 3: In the "Tethering" page, ensure that "Bluetooth" is enabled (box filled)
At this point, you should see an ip address in the top left corner. 
![EV3 Bluetooth enabled with ip address](./images/bluetooth-tethering.jpg)

### 4: Go back to the "Wireless and Networks" menu and navigate to "Bluetooth"
![EV3 Bluetooth page](./images/bluetooth.jpg)

### 5: In the "Bluetooth" page, ensure that "Powered" and "Visible" are both enabled (boxes filled)
![EV3 Bluetooth powered and visible](./images/powered-and-visible.jpg)

### 6: On your computer, navigate to "pair device" in the Bluetooth menu
![PC bluetooth menu](./images/kde-pair-device.png)

### 7: In the "Select Device" menu, double click "ev3dev" when it appears
![PC Select Device menu](./images/kde-select-ev3dev.png)

### 8: You should see a prompt on the brick and computer to confirm a passkey.
If they match, press "Matches" on the computer and "Accept" on the robot

![PC confirm passkey](./images/confirm-passkey-pc.png)
![EV3 confirm passket](./images/confirm-passkey-robot.jpg)

### 9: On the computer, you should see a "connecting" and a "setup failed" screen
Don't worry about the "setup failed" screen, this is completely normal. 
You can close it out as we will not be needing it anymore.

![PC connecting Screen](./images/kde-connecting-screen.png)
![PC setup failed screen](./images/kde-connection-failed.png)

### 10: In your wifi menu, you should see "ev3dev Network" under "Available"
Press "Connect"

![PC network menu](./images/kde-connect-PAN.png)

### 11: When you press "Connect", you should see this menu on the EV3 brick:
Press accept

![EV3 confirm connection](./images/accept-connection.jpg)

### 12: You are now connected! To test your connection, you can ssh into the robot with the ip address in the top left corner
The password is "maker"

![PC ssh into robot](./images/ssh-success.png)

### Use steps 10-12 when re-connecting in the future.
