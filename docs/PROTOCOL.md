# Websocket Protocol

The communication between the client app served by the esp and the websocket is binary and allows for star tracker control

## Commands
A command is each 2 bytes in MSB

`[14 bit: Action Values] [2 bit: Action]`

### Action
 
[//]: # (- `00`: **Request info**)
- `01`: **Move with speed and direction (or stop)**
- `10`: **Set tracking**

[//]: # (- `11`: **Set setting**)


### Move with speed and direction (or stop)
`[13 bit: Speed] [1 bit: direction] 01`

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

`[14 bit: Enable/disable] 10`

#### Enable/Disable
- `0`: Disable
- `1`: Enable

Example command for enabling tracking
`0b 0000 0000 0000 0110` or `0x6`


## Responses
A response from the server is each 2 bytes in MSB

`[14 bit: Values] [2 bit: Type]`

### Response types

- `00`: **All movement stopped**
- `01`: **Constant movement started**
- `10`: **Tracking started**
- `11`: **Tracker height changed**

### All movement stopped
All movement (constant and tracking) has stopped. Tracker is not busy anymore

`0000 0000 0000 0000`

### Constant movement started
Tracker moves in a direction in constant speed

`[13 bit: Speed] [1 bit: direction] 01`

### Tracking started
Star tracking has started with dynamic speed

`0000 0000 0000 0010`

#### Exception
Calibration is ongoing when third bit is set

`0000 0000 0000 0110`

### Tracker height changed
Notification that the height has changed. 
- 0% = Tracker is on the bottom 
- 100% = Tracker is at the top and can't go any further, it needs to be repositioned 

`0000 000 [7 bits: height in percentage] 11`



