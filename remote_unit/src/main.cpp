#ifdef WEMOS_D1_MINI
#define BLYNK_PRINT Serial
#endif

#include <Arduino.h>
#include <ESP8266WiFi.h>    
#include <FastLED.h>
#include <SoftwareSerial.h>
#include <BlynkSimpleEsp8266.h>
#include <InputDebounce.h>
#include <time.h>


#define BUTTON_DEBOUNCE_DELAY   20   // Milliseoncds
#define NUM_LEDS 60

#ifdef WEMOS_D1_MINI
const int LED_STRIP = D3;     // Connect to data pin of Neopixel LED strip
const int INTERCOM_RX = D2;   // Connect to the TX pin of the HC-12
const int INTERCOM_TX = D1;   // Connect to the RX pin of the HC-12
#else
const int LED_STRIP = 0;
const int INTERCOM_RX = 3;    // Connect to the TX pin of the HC-12
const int INTERCOM_TX = 1;    // Connect to the RX pin of the HC-12
#endif


#ifdef BUZZER_ENABLED
  #ifdef WEMOS_D1_MINI
  const int BUZZER = D0;
  #endif
#endif

#ifdef OPEN_DOOR_BUTTON_ENABLED
  #ifdef WEMOS_D1_MINI
  const int OPEN_DOOR = D5;     // Connect to ground when a button is pushed
  #endif
#endif

const int MY_ADDRESS = 12;
const int SILENT_MODE_COLOR  = CRGB::Blue;

///////////////////////////////////////////////////////////////////////////////
// SSID of the WiFi network to connect.
//
const char *ssid = "WiFi SSID here"; 

///////////////////////////////////////////////////////////////////////////////
// Password required to connect to the WiFi network.
//
const char *wifiPassword = "WiFi password here";  


///////////////////////////////////////////////////////////////////////////////
// Blynk token.
//
const char *blynkToken = "Blynk token here";

///////////////////////////////////////////////////////////////////////////////
// Message codes
// 

// Sent from intercoms to call the main switchboard.
#define MSG_CALL_TO_MAIN_SWITCHBOARD         8

// Sent from intercoms to call the caretaker phone (K).
#define MSG_CALL_CARETAKER_DOOR_ENTRY_PHONE  9

// Open building's entry door (AP). Sent when the intercom's open door button 
// is pushed. User ID tells who's opening the door.
#define MSG_OPEN_DOOR                       16

// Sent when the user hooks off the intercom's handset.
#define MSG_HOOK_OFF                        17

// Sent when the user hooks on the intercom's handset.
#define MSG_HOOK_ON                         18

// Send from intercoms to call the secondary switchboard. In model the 
// secondary button can be configured to send this message by setting the JP1
// jumper to position C.
#define MSG_CALL_TO_SECONDARY_SWITCHBOARD   19

// Turns on the video camera and the intercom's screen (AI).
#define MSG_CAMERA_ON                       20

// Sent when the external switch connected to the CFP terminals is closed.
#define MSG_CALL_FROM_FLOOR_DOOR            21

// Sent by one intercom to call other intercomms (INT).
#define MSG_CALL_INTERCOM                   24

// After sending MSG_CALL_INTERCOM three consecutive
// MSG_CALL_INTERCOM_RESPONSE are sent.
#define MSG_CALL_INTERCOM_RESPONSE          26

// Activates a generic actuator. In model the secondary button can be
// configured to send this message by setting the JP1 jumper to position A.
#define MSG_GENERIC_ACTUATOR                29

// High prioritary call to main switchboard (PAN).
#define MSG_HIGH_PRIO_CALL_TO_MAIN_SWITCHBOARD  30

// Sent from switchboard for calling an intercom.
#define MSG_CALL_FROM_SWITCHBOARD_1         32

// After sending this message I got a MSG_HOOK_ON, but only once. What does
// this mean?.
#define MSG_UNKNOWN_1                       33


#define MSG_CALL_FROM_SWITCHBOARD_2             37
#define MSG_CALL_FROM_SWITCHBOARD_3             42
#define MSG_CALL_FROM_SWITCHBOARD_4             43
#define MSG_CALL_FROM_SWITCHBOARD_5_SCREEN_ON   45


// Sent when someone calls at the building's entry. The ring tone is played
// once per each message, this message is usually sent two times, followed by 
// MSG_CALL_FROM_ENTRY_DOOR_SCREEN_ON.
#define MSG_CALL_FROM_ENTRY_DOOR            48

// Sent when the screen is turned off ?????
#define MSG_SCREEN_OFF                      49

// Similar to MSG_RING_TONE but also makes the intercom turn on the video 
// screen.
#define MSG_CALL_FROM_ENTRY_DOOR_SCREEN_ON  50


#define MSG_START_BLINKING_OPEN_DOOR_BTN    51
#define MSG_STOP_BLINKING_OPEN_DOOR_BTN     52


///////////////////////////////////////////////////////////////////////////////
// Global variables
// 
WidgetTerminal terminal(V2);

SoftwareSerial intercom(INTERCOM_RX, INTERCOM_TX);

CRGB leds[NUM_LEDS];

InputDebounce inputButton;


// Last time the door was opened, either with Blynk or the input button.
unsigned long lastOpenDoorTime;

// While in silent mode the buzzer is disabled and only the leds are
// turned on if someone rings the door.
bool silentMode;

// True if the input button has remained pressed for more than 1s, goes
// to False again once the button has been released.
bool longTap;


void notifyCall(const CRGB &color)
{
  #ifdef BUZZER_ENABLED
  if (!silentMode)
    digitalWrite(BUZZER, HIGH);
  #endif

  fill_solid(leds, NUM_LEDS, color);
  FastLED.show(); 
  
  delay(500);
  
  #ifdef BUZZER_ENABLED
  digitalWrite(BUZZER, LOW);
  #endif

  if (silentMode)
    fill_solid(leds, NUM_LEDS, SILENT_MODE_COLOR);
  else 
    fill_solid(leds, NUM_LEDS, CRGB::Black);

  FastLED.show();
}


void transmitMessage(byte msgCode, byte msgAddr)
{
  byte checksum = 0;

  checksum += __builtin_popcount(msgCode);
  checksum += __builtin_popcount(msgAddr);
    
  intercom.write(msgCode << 6);
  intercom.write((msgAddr << 4) | (msgCode >> 2));
  intercom.write((checksum << 4) | (msgAddr >> 4));
  
  #ifdef SERIAL_OUTPUT
  Serial.print("TX -> code: ");
  Serial.print(msgCode);
  Serial.print(" address: ");
  Serial.println(msgAddr);
  #endif

  terminal.print("TX -> code: ");
  terminal.print(msgCode);
  terminal.print(" address: ");
  terminal.println(msgAddr);
  terminal.flush();
}

void processMessage(byte msgCode, byte msgAddr) 
{
  #ifdef SERIAL_OUTPUT
  Serial.print("RX <- code: ");    
  Serial.print(msgCode, DEC);
  Serial.print(" address: ");
  Serial.println(msgAddr, DEC);
  #endif

  terminal.print("RX <- code: ");    
  terminal.print(msgCode, DEC);
  terminal.print(" address: ");
  terminal.println(msgAddr, DEC);
  terminal.flush();

  unsigned long currentTime = millis();

  if (msgAddr == MY_ADDRESS) {
    switch (msgCode)
    {
    case MSG_CALL_TO_SECONDARY_SWITCHBOARD:
    case MSG_CALL_FROM_ENTRY_DOOR:
      notifyCall(CRGB::Red);
      break;
    case MSG_CALL_FROM_ENTRY_DOOR_SCREEN_ON:
      // If I opened the door less than 60 seconds ago, open
      // the door automatically if another call occurs.
      if (lastOpenDoorTime > 0 && currentTime - lastOpenDoorTime < 60000) {
        // Wait for 2.5 seconds before opening the door. 
        delay(2500);
        transmitMessage(MSG_OPEN_DOOR, MY_ADDRESS);
      } else {
        Blynk.notify("Someone is calling at the door");
        notifyCall(CRGB::Red);
      }
      break;
    case MSG_OPEN_DOOR:
      lastOpenDoorTime = currentTime;
      break;
    }
  }
}

void toggleSilentMode() {
  silentMode = !silentMode;
  if (silentMode) {
    #ifdef SERIAL_OUTPUT
    Serial.println("Silent mode: on");
    #endif
    fill_solid(leds, NUM_LEDS, SILENT_MODE_COLOR);
    FastLED.show(); 
  } else {
    #ifdef SERIAL_OUTPUT
    Serial.println("Silent mode: off");
    #endif
    fill_solid(leds, NUM_LEDS, CRGB::Black);
    FastLED.show();
  }
}

void buttonPressedDuration(uint8_t pinIn, unsigned long duration)
{
  // Toggle silent mode when the button remains tapped for more than 1s.
  if (!longTap && duration > 1000) {
    longTap = true;
    toggleSilentMode();
  }
}

void buttonReleasedDuration(uint8_t pinIn, unsigned long duration)
{  
  if (duration < 1000) {
      transmitMessage(MSG_OPEN_DOOR, MY_ADDRESS);
      lastOpenDoorTime = millis();
  }

  longTap = false;
}

void setup() 
{
  #ifdef SERIAL_OUTPUT
  Serial.begin(115200);
  delay(1000);
  #endif

  #if BUZZER_ENABLED
  pinMode(BUZZER, OUTPUT);
  #endif

  #if OPEN_DOOR_BUTTON_ENABLED
  inputButton.registerCallbacks(
    NULL, NULL, buttonPressedDuration, buttonReleasedDuration);

  inputButton.setup(
    OPEN_DOOR, 
    BUTTON_DEBOUNCE_DELAY, 
    InputDebounce::PIM_INT_PULL_UP_RES);
  #endif

  pinMode(LED_BUILTIN, OUTPUT);

  intercom.begin(4800);

  Blynk.begin(blynkToken, ssid, wifiPassword);

  lastOpenDoorTime = 0;

  FastLED.addLeds<NEOPIXEL, LED_STRIP>(leds, NUM_LEDS);
  FastLED.setBrightness(120);

  pinMode(LED_STRIP, OUTPUT_OPEN_DRAIN);
  
  notifyCall(CRGB::Red);
  digitalWrite(LED_BUILTIN, HIGH);
  delay(500);
  digitalWrite(LED_BUILTIN, LOW);

  notifyCall(CRGB::Green);
  digitalWrite(LED_BUILTIN, HIGH);
  delay(500);
  digitalWrite(LED_BUILTIN, LOW);

  notifyCall(CRGB::Blue);
  digitalWrite(LED_BUILTIN, HIGH);
  delay(500);
  digitalWrite(LED_BUILTIN, LOW);
}

// 
// Receive commands sent from the Blynk app via V0.
//
BLYNK_WRITE(V0) {
  byte msgCode = param.asInt();
  if (msgCode != 0) {
    transmitMessage(msgCode, MY_ADDRESS);
    if (msgCode == MSG_OPEN_DOOR)
      lastOpenDoorTime = millis();
  }
}


void loop()
{
  Blynk.run();

  inputButton.process(millis());

  if (intercom.available())
  {
    byte msg[3];
    intercom.readBytes(msg, 3);

    byte expectedChecksum = msg[2] >> 4;
    byte msgAddr = (msg[2] << 4) | (msg[1] >> 4);
    byte msgCode = ((msg[1] << 4) | (msg[0] >> 4)) >> 2;
    byte actualChecksum = 0;
    
    actualChecksum += __builtin_popcount(msgCode);
    actualChecksum += __builtin_popcount(msgAddr);
  
    if (msgCode != 0 && actualChecksum == expectedChecksum) {
      processMessage(msgCode, msgAddr);
    } 
  }
}
