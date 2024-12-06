/*
 * ----------------------------------
 *             MFRC522      Node
 *             Reader/PCD   MCU
 * Signal      Pin          Pin
 * ----------------------------------
 * RST/Reset   RST          D1 (GPIO5)
 * SPI SS      SDA(SS)      D2 (GPIO4)
 * SPI MOSI    MOSI         D7 (GPIO13)
 * SPI MISO    MISO         D6 (GPIO12)
 * SPI SCK     SCK          D5 (GPIO14)
 * 3.3V        3.3V         3.3V
 * GND         GND          GND
 */

#include <ESP8266WiFi.h>
#include <ESP8266mDNS.h>

#include <SPI.h>
#include <MFRC522.h>

const char *WIFI_SSID = "HUAWEI-2.4G-E75z";
const char *WIFI_PASSWORD = "JgY5wBGt";
const char *HOST_NAME = "uets-rfid-reader";

const int SERVER_PORT = 8888;

const uint8_t RST_PIN = 5;
const uint8_t SS_PIN = 4;

WiFiServer server(SERVER_PORT);
MFRC522 rfid(SS_PIN, RST_PIN);

void server_setup()
{
  WiFi.hostname(HOST_NAME);
  WiFi.mode(WIFI_STA);
  WiFi.begin(WIFI_SSID, WIFI_PASSWORD);

  Serial.println("Connecting to Wifi");
  while (WiFi.status() != WL_CONNECTED)
  {
    delay(500);
    Serial.print(".");
    delay(500);
  }
  Serial.println("Connected to Wifi");
  Serial.println(WiFi.localIP());

  if (!(MDNS.begin(HOST_NAME)))
  {
    Serial.println("Error setting up mDNS");
  }
  Serial.println("mDNS Started");

  server.begin();
}

void rfid_setup()
{
  SPI.begin();
  rfid.PCD_Init();
}

void setup()
{
  Serial.begin(9600);

  server_setup();
  rfid_setup();
}

void loop()
{
  if (!MDNS.update())
  {
    Serial.println("Failed to update mDNS");
  }

  WiFiClient client = server.available();

  if (client)
  {
    if (client.connected())
    {
      Serial.println("Client Connected");
    }

    while (client.connected())
    {
      if (rfid.PICC_IsNewCardPresent() && rfid.PICC_ReadCardSerial())
      {
        String content = "";
        for (byte i = 0; i < rfid.uid.size; i++)
        {
          content += rfid.uid.uidByte[i] < 0x10 ? "0" : "";
          content += String(rfid.uid.uidByte[i], HEX);
        }

        client.write(content.c_str());
        client.write("\n");

        Serial.println("RFID written to client");

        rfid.PICC_HaltA();
        rfid.PCD_StopCrypto1();
      }
    }

    client.stop();
    Serial.println("Client disconnected");
  }
}
