# Websocket Protocol

The communication between the client app served by the esp and the websocket is binary and allows for star tracker control

## Commands
A command is each 4 bytes in MSB

`[28 bit: Action Values] [4 bit: Action]`

### Action
 
- `0000`: **Start calibration**
- `0001`: **Move with speed and direction (or stop)**
- `0010`: **Set tracking**
- `0011`: **Request state**
- `0100`: **Camera test capture**
- `0101`: **Start camera capture interval**
- `0111`: **Stop camera capture interval**

[//]: # (- `11`: **Set setting**)

#### Calibration
Start calibration at any given time

`0000 0000 0000 0000`

### Move with speed and direction (or stop)
`[27 bit: Speed] [1 bit: direction] 0001`

#### Direction
- `0`: Down
- `1`: Up

#### Speed
Steps per second 0 - 6400. 

One rotation per second is 3200 steps. To stop the movement, set the speed to 0.

Example command for 3200 steps per second:
`0b 0110 0100 0000 0101` or `0x64 0x05`

### Set tracking
Enable or disable tracking

`[27 bit: unused] [1 bit: Enable/disable] 0010`

#### Enable/Disable
- `0`: Disable
- `1`: Enable

Example command for enabling tracking
`0b 0000 0000 0000 0110` or `0x6`

### Request state
Request the tracker to send its state 

`0000 0000 0000 0011`

### Camera test capture
Fire single test capture

`0000 0000 0000 0100`

### Start camera capture interval
Start interval image capturing

Time interval (shutter speed) is provided in milliseconds. 
Once capturing is started it cannot be restarted/changed with another start command. 
It has to be stopped first using the stop command.

Example: 20" results in 20'000 ms

`[28 bit: timer interval] 0101`

### Stop camera capture interval
Stop interval image capturing

`[28 bit: unused] 0111`


---

## Responses
A response from the server is each 4 bytes in MSB

`[28 bit: Values] [2 bit: Type]`

### Response types

- `0000`: **All movement stopped**
- `0001`: **Constant movement started**
- `0010`: **Tracking started**
- `0011`: **Tracker height changed**
- `0100`: **Calibration started**
- `0101`: **Image captured**
- `0110`: **Camera capturing changed**
- `1111`: **State message**

### All movement stopped
All movement (constant and tracking) has stopped. Tracker is not busy anymore

`0000 0000 0000 0000`

### Constant movement started
Tracker moves in a direction in constant speed

`[13 bit: Speed] [1 bit: direction] 01`

### Tracking started
Star tracking has started with dynamic speed

`0000 0000 0000 0010`


### Tracker height changed
Notification that the height has changed. 
- 0% = Tracker is on the bottom 
- 100% = Tracker is at the top and can't go any further, it needs to be repositioned 

`0000 000 [7 bits: height in percentage] 11`

### Tracking started
Star tracking has started calibrating

`0000 0000 0000 0100`
 
It responds with "All movement stopped" once finished

### Image captured
An image capture has been triggered

`0000 0000 0000 0101`

### Camera capturing changed
Automatic camera capturing has been started or stopped

`[27 bit zero] [1 bit: state] 0110`

Where states are:
- `0`: Off
- `1`: On

### State message
Returns the star trackers current state 

`[26 bit zero] [2 bit: state] 1111`

Following states are possible:
- `00`: Idle
- `01`: Calibrating
- `10`: Moving
- `11`: Tracking





