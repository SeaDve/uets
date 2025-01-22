#include <ESP8266WiFi.h>
#include <ESP8266mDNS.h>
#include <ESP8266WebServer.h>

const char *WIFI_SSID = "HUAWEI-2.4G-E75z";
const char *WIFI_PASSWORD = "JgY5wBGt";
const char *HOST_NAME = "uets-relay";

const int SERVER_PORT = 8888;

const uint8_t LED_PIN = D2;
const uint8_t RELAY_PIN = D3;

ESP8266WebServer server(SERVER_PORT);

void handle_high()
{
    Serial.println("Handling high");

    digitalWrite(LED_PIN, HIGH);
    digitalWrite(RELAY_PIN, HIGH);
    server.send(200);
}

void handle_low()
{
    Serial.println("Handling low");

    digitalWrite(LED_PIN, LOW);
    digitalWrite(RELAY_PIN, LOW);
    server.send(200);
}

void handle_status()
{
    Serial.println("Handling status");

    int value = digitalRead(RELAY_PIN);
    server.send(200, "text/plain", String(value));
}

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

    server.on("/high", handle_high);
    server.on("/low", handle_low);
    server.on("/status", handle_status);

    server.begin();
    Serial.println("Server started");
}

void setup()
{
    Serial.begin(9600);

    pinMode(LED_PIN, OUTPUT);
    pinMode(RELAY_PIN, OUTPUT);

    server_setup();
}

void loop()
{
    if (!MDNS.update())
    {
        Serial.println("Failed to update mDNS");
    }

    server.handleClient();
}
