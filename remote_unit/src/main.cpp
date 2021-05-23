#include <Arduino.h>
#include <ArduinoJson.h>
#include <ESP8266WiFi.h>    
#include <FastLED.h>
#include <SoftwareSerial.h>
#include <Ticker.h>
#include <AsyncMqttClient.h>

#define NUM_LEDS 60

const int BUZZER = D0;
const int LED_STRIPE = D3;
const int INTERCOM_RX = D2;    // Connect to the TX pin of the HC-12
const int INTERCOM_TX = D1;    // Connect to the RX pin of the HC-12
const int OPEN_DOOR = D5;      // Connected to ground when a button is pushed.


const int MY_ADDRESS = 12;


///////////////////////////////////////////////////////////////////////////////
// SSID of the WiFi network to connect.
//
const char *ssid = "WiFi SSID here"; 

///////////////////////////////////////////////////////////////////////////////
// Password required to connect to the WiFi network.
//
const char *wifiPassword = "WiFi password here";  

///////////////////////////////////////////////////////////////////////////////
// MQTT settings.
//
const char* mqttServer = "broker.emqx.io";
const int mqttPort = 1883;
const char* mqttUser = "emqx";
const char* mqttPassword = "public";

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


SoftwareSerial intercom(INTERCOM_RX, INTERCOM_TX);

CRGB leds[NUM_LEDS];

WiFiClient wifiClient;
WiFiEventHandler wifiConnectHandler;
WiFiEventHandler wifiDisconnectHandler;

AsyncMqttClient mqttClient;

Ticker wifiReconnectTimer;
Ticker mqttReconnectTimer;

unsigned long lastOpenDoorTime;

void connectToWifi() {
  Serial.println("Connecting to Wi-Fi...");
  WiFi.begin(ssid, wifiPassword);
}

void connectToMqtt() {
  Serial.println("Connecting to MQTT...");
  mqttClient.connect();
}

void signalCall()
{
  digitalWrite(BUZZER, HIGH);
  fill_solid(leds, NUM_LEDS, CRGB::Red);
  FastLED.show(); 
  delay(1000);
  digitalWrite(BUZZER, LOW);
  fill_solid(leds, NUM_LEDS, CRGB::Black);
  FastLED.show();
  delay(500);
}

void transmitMessage(byte msgCode, byte msgAddr)
{
  byte checksum = 0;

  checksum += __builtin_popcount(msgCode);
  checksum += __builtin_popcount(msgAddr);
    
  intercom.write(msgCode << 6);
  intercom.write((msgAddr << 4) | (msgCode >> 2));
  intercom.write((checksum << 4) | (msgAddr >> 4));

  Serial.print("TX -> code: ");
  Serial.print(msgCode);
  Serial.print(" address: ");
  Serial.println(msgAddr);
}

void onWifiConnect(const WiFiEventStationModeGotIP& event) {
  Serial.print("Connected to Wi-Fi, IP:");
  Serial.println(WiFi.localIP());
  connectToMqtt();
}

void onWifiDisconnect(const WiFiEventStationModeDisconnected& event) {
  Serial.println("Disconnected from Wi-Fi.");
  // Ensure we don't reconnect to MQTT while reconnecting to Wi-Fi
  mqttReconnectTimer.detach(); 
  wifiReconnectTimer.once(2, connectToWifi);
}

void onMqttConnect(bool sessionPresent) {
  Serial.println("Connected to MQTT.");
  Serial.print("Session present: ");
  Serial.println(sessionPresent);
  
  packetIdSub = mqttClient.subscribe("simplebus2/intercom", 0);
  Serial.print("Subscribing at QoS 2, packetId: ");
  Serial.println(packetIdSub);

  // Turn off the built-in led, which indicates that we connected
  // successfully to MQTT. Notice that if LED_BUILTIN is HIGH the
  // led is OFF.
  digitalWrite(LED_BUILTIN, HIGH);
}

void onMqttDisconnect(AsyncMqttClientDisconnectReason reason) {
  Serial.print("Disconnected from MQTT. Reason: ");
  Serial.println((int )reason);
  if (WiFi.isConnected()) {
    mqttReconnectTimer.once(2, connectToMqtt);
  }
}

void onMqttSubscribe(uint16_t packetId, uint8_t qos) {
  Serial.println("Subscribe acknowledged.");
  Serial.print("  packetId: ");
  Serial.println(packetId);
  Serial.print("  QoS: ");
  Serial.println(qos);
}

void onMqttUnsubscribe(uint16_t packetId) {
  Serial.println("Unsubscribe acknowledged.");
  Serial.print("  packetId: ");
  Serial.println(packetId);
}

void onMqttMessage(char* topic, char* payload, AsyncMqttClientMessageProperties properties, size_t len, size_t index, size_t total) {
  DynamicJsonDocument doc(200);
  DeserializationError error = deserializeJson(doc, payload);
 
  if (error) {
    Serial.print(F("deserializeJson() failed: "));
    Serial.println(error.f_str());
    return;
  }

  byte msgCode = doc["code"];
  byte msgAddr = doc["address"];

  transmitMessage(msgCode, msgAddr);
}

void onMqttPublish(uint16_t packetId) {
  Serial.println("Publish acknowledged.");
  Serial.print("  packetId: ");
  Serial.println(packetId);
}

void setup() 
{
  Serial.begin(115200);
  delay(1000);

  pinMode(LED_BUILTIN, OUTPUT);
  pinMode(BUZZER, OUTPUT);
  pinMode(OPEN_DOOR, INPUT_PULLUP);
  
  FastLED.addLeds<NEOPIXEL, LED_STRIPE>(leds, NUM_LEDS);
  FastLED.setBrightness(120);

  intercom.begin(4800);

  wifiConnectHandler = WiFi.onStationModeGotIP(onWifiConnect);
  wifiDisconnectHandler = WiFi.onStationModeDisconnected(onWifiDisconnect);

  mqttClient.onConnect(onMqttConnect);
  mqttClient.onDisconnect(onMqttDisconnect);
  mqttClient.onSubscribe(onMqttSubscribe);
  mqttClient.onUnsubscribe(onMqttUnsubscribe);
  mqttClient.onMessage(onMqttMessage);
  mqttClient.onPublish(onMqttPublish);
  mqttClient.setServer(mqttServer, mqttPort);

  connectToWifi();

  lastOpenDoorTime = 0;
}


void loop()
{
  unsigned long currentTime = millis();

  if (digitalRead(OPEN_DOOR) == LOW) {
    transmitMessage(MSG_OPEN_DOOR, MY_ADDRESS);
    lastOpenDoorTime = currentTime;
    delay(1000);
  }

  if (intercom.available()) {
    byte msg[3];
    intercom.readBytes(msg, 3);

    byte expectedChecksum = msg[2] >> 4;
    byte msgAddr = (msg[2] << 4) | (msg[1] >> 4);
    byte msgCode = ((msg[1] << 4) | (msg[0] >> 4)) >> 2;
    byte actualChecksum = 0;
    
    actualChecksum += __builtin_popcount(msgCode);
    actualChecksum += __builtin_popcount(msgAddr);
  
    if (msgCode != 0 && actualChecksum == expectedChecksum) {
      Serial.print("RX <- code: ");    
      Serial.print(msgCode, DEC);
      Serial.print(" address: ");
      Serial.print(msgAddr, DEC);
      Serial.print(" checksum: ");
      Serial.println(expectedChecksum, DEC);
      
      if (msgAddr == MY_ADDRESS) {
        switch (msgCode)
        {
        case MSG_RING_TONE:
        case MSG_RING_TONE_SCREEN_ON:
          // If I opened the door less than 60 seconds ago, open
          // the door automatically if another call occurs.
          if (lastOpenDoorTime > 0 && currentTime - lastOpenDoorTime < 60000) {
            if (msgCode == MSG_RING_TONE_SCREEN_ON) {
              delay(3000);
              transmitMessage(MSG_OPEN_DOOR, MY_ADDRESS);
            }
          } else {
            signalCall();
          }
          break;
        case MSG_OPEN_DOOR:
          lastOpenDoorTime = currentTime;
          break;
        }
      }
    } 
  }
}
