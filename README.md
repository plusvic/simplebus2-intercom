Overview
--------

Simplebus2 is a two-wire door entry system created by the Italian company Comelit. It supports both audio and video, and it's widely used in residential buildings, specially across Europe.

The building where I live has one of those systems, so I wanted to understand how it worked in order to solve a practical problem: getting notified when someone is ringing my doorbell. I work from home and usually wear noise-cancelling headphones while working, which means that I can't hear the doorbell. I wanted to install a complementary notification system, such as a light that turns on, or something that vibrates on my desk.

In this project I'm sharing my findings about how the Simplebus2 system, and a detailed description of the devices I've built.

How it works
------------

The Simplebus2 door entry system uses only two wires for carrying both DC power and signals to the intercom devices installed on each apartment. All the devices share the same two wires, hence the "bus" word in the name. Those two wires are not polarized, they are labeled the same way (L) and can be connected arbitrarily to the intercom devices. Of course, being DC there's a GND and a Vcc, but devices are prepared to accept both wires in any of the two possible ways. This is done by putting a full-bridge rectifier at the device's input.

The DC voltage in the bus is 22-34V according to the manufacturer (in my particular case it is 26.5V), and signals are mounted on top of this DC offset. When someone calls an apartment from the building's entry door, a message is transmitted over the bus. All the intercom units in the building can see the message, but only the unit in the target apartment reacts to the message. Each intercom unit has its own 8-bits address, which is configured via a 8-way DIP switch during installation. See the interior of a Comelit 2738W intercom with the DIP switch in red:

![9254781621186832244](https://user-images.githubusercontent.com/182937/119274313-8b263380-bc0f-11eb-9708-aa4b1010644b.jpeg)


Multiple units can share the same address, this is common practice when two or more intercoms are installed in the same apartment. Simplebus2 messages are 18-bits long, structured as follows:

* 6 bits for the message code

* 8 bits for the intercom address

* 4 bits for the checksum

Messages are transmitted using pulse length encoding, where each bit consists in a burst of pulses at 25kHz during 3ms, followed by a silence that can be 3ms or 6ms long. The length of the silence determines the bit's value, 3ms for zero, 6ms for one. Messages start with a preamble consisting in the 3ms burst followed by a 17ms silence. The first bit is transmitted right after the preamble.

![5891691621185479515](https://user-images.githubusercontent.com/182937/119273630-13a2d500-bc0c-11eb-9ebe-9dacef47744a.png)

The image above shows the capture of a message with an oscilloscope, the yellow stripes are the 25kHz bursts, which have always the same width because they are all 3ms long. You can also see that the space in between the bursts are not homogeneous, some spaces are wider than others, the wider ones (6ms) represent the 1s, the narrower (3ms) are 0s. The preamble can be also clearly appreciated. 

The bits in green correspond to the message code, the ones in red are the intercom's address, and the checksum is shown in yellow. The most significant bit (MSB) is sent first, so the actual values are:

Message code:  110000 = 48

Address: 00001100 = 12

Checksum: 0100 = 4

The checksum is simply the number of bits that are set to 1 in the message code and address. In this case the checksum is 4 because there are four bits set, two in the message code and two in the address. Notice how the address matches the position of the DIP switches in the first picture, and curiously enough the most significant bits in the switch are at the right, so the order of the bits is identical in the switch and in the oscilloscope capture. I live at the 6th floor of a building with two apartments per floor, so it make sense that my intercom has address 12, if the installer was being meticulous my next door neighbor should have address 11, and the apartment just below mine should have address 10.

Right after the message (at the right end of the oscilloscope capture) you can see four bursts separated by 3ms silences, those are not part of the message itself, that's a response from the target intercom acknowledging that the message has been received. The amplitude of the acknowledgement signal is clearly higher because the source of that signal is the intercom in my house, which is close to the point where the oscilloscope was connected, while the message comes from the building's entry door, which is further away and therefore arrives attenuated.

These are some of the supported message codes, but bear in mind that some descriptions may be inaccurate as I've have deduced their meaning based on experimentation.

| Code     | Description                                                                                      |
|----------|--------------------------------------------------------------------------------------------------|
| 16 | Open door. Sent from intercoms to the building's entry door when you pulse the open door button.       |
| 17 | Handset hook off |
| 18 | Handset hook on. |
| 19 |	Call switchboard.	|
| 20 |	Turn on door camera and video screen.	|
| 21 | Floor door's ring tone. Intercoms have a pair of terminals labeled CFP (which are visible in the picture above), these terminals are for connecting a push button at your apartment's door. This message is sent by the intercom when it detects that its CFP terminals are connected together, so that all the other intercom's installed in the apartment start ringing. |
| 48 | Building door's ring tone. Send from the building's entry door when someone is calling your apartment. For each of these messages the intercom rings once, the full calling sequence is two messages with code 48 followed by a message with code 50. |
| 50 | Building door's ring tone and end of call. When this message is received the intercom ring in the same way it does with message 48, but also turns on the video screen. |

The devices
-----------

Once I understood how the Simplebus2 protocol worked, it was time for building a device that could notify me when someone ring my doorbell. The requisites were:

* The device should be powered from the bus in the same way the real intercom does. 

* It should fit in some empty space inside a Comelit 2738W casing.  

* Any notification sent by the device should be transmitted wireless to some other receiving device close to my desk.

The first two requisites were easy to fulfill, the bus provides enough power for the Comelit 6701W video intercom,  which consumes 5.4W according to its datasheet, our device should a require just a fraction of that. Also, the Comelit 2738W, has 30x60x15mm empty cavity that looked big enough for holding my device.

The third requisite was the one that needed more experimentation. I created a first prototype based in a [433MHz transmitter/receiver kit](https://randomnerdtutorials.com/rf-433mhz-transmitter-receiver-module-with-arduino/) with the transmitter inside the intercom sending the signals seen in the bus to the receiver in my desk, connected to an Arduino board. But this was unidirectional, I could receive messages from the intercom, but couldn't send messages back, for example for opening the door directly from my desk. I wanted a bi-directional communication channel, so I tried an [ESP8266](https://en.wikipedia.org/wiki/ESP8266) connected to my WiFi network, but the ESP8266 was very power-hungry. Finally a resorted to a pair of [HC-12](https://howtomechatronics.com/tutorials/arduino/arduino-and-hc-12-long-range-wireless-communication-module/) wireless serial communication modules that worked like a charm. 

The final setup was:

* A simple circuit with a PIC12F508 decodes the Simplebus2 messages and send them via UART to an HC-12 module. The HC-12 module transmit the message wirelessly. All of this stuff was put into the empty cavity of my Comelit 2738W intercom.

* At the other end (in my desk) another HC-12 module receives the wireless message and passes it to a [Wemos D1 mini](https://www.wemos.cc/en/latest/d1/d1_mini.html) board.

* The Wemos D1 mini controls a led strip that flashes when someone is calling my door and a haptic motor that produces some vibration. 

* Messages can also travel the other way around from the Wemos D1 mini to the PIC12F508 via the two HC-12 modules, and the PIC12F508 produces the signals that are transmitted over the bus.

The circuit that decodes the Simplebus2 messages is shown below:

![8994681621349035928](https://user-images.githubusercontent.com/182937/119274024-1f8f9680-bc0e-11eb-83b6-ef4311cebe2d.png)

The bridge rectifier (DB1) fixes the polarity of the input, so that the Simplebus2 wires can be connected in any way without having to take their polarity into account. A low-pass filter (R1 and C1) removes the voltage fluctuations produced by signals and feeds the base DC to the LM7805 voltage regulator that provides 5V to the other components. A high-pass filter (R2 and C3) removes the DC component on the Simplebus2 line allowing the signals pass to the non-inverting input of a LM2903 comparator. The comparator's inverting input receives a reference voltage of around 500mV produced by the voltage divider (R3 and R4).

The comparator's output is fed into the PIC12F508 microcontroller, which decodes the messages and re-transmitted via UART to the HC-12 transceiver module. The Q1 transistor is used for transmitting messages through the bus, it is usually in cutoff mode, but the microcontroller allows current to pass through the transistor for brief intervals causing the voltage changes in the bus that will conform the message. The program that runs inside the PIC12F508 can be found [here](repeater/simplebus2-repeater.asm).


