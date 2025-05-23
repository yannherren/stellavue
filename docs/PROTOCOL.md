# Websocket Protocol

The communication between the client app served by the esp and the websocket is binary and allows for star tracker control

## Commands
A command is each 2 bytes in MSB

`[14 bit: Action Values] [2 bit: Action]`

### Action
 
- `00`: **Request info**
- `01`: **Move with speed and direction (or stop)**
- `10`: **Set tracking**
- `11`: **Set setting**


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



