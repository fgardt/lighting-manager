# **Lighting Manager**

Control your ws281x LED strip with a Raspberry Pi through an API.

---

## **Setup**

### **Docker**

This can be run with `docker-compose up -d lighting-manager` with the following compose file:

```yml
version: "3"

services:
  lighting-manager:
    ports:
      - "88:88"
    privileged: true
    image: ghcr.io/fgardt/lighting-manager:latest
    container_name: lighting-manager
    restart: unless-stopped
    command: ["--log-level", "info", "--count", "YOUR_LED_COUNT", "--pin", "YOUR_PIN"]
```

_note: the container needs to be run in privileged mode to access the GPIOs._

### **Standalone**

Another option would be to either compile the code yourself or download the binary from the latest [release](https://github.com/fgardt/lighting-manager/releases).\
Once you have the binary simply execute it (and pass along all required flags described below).

_note: this will probably need to be run as root or with `sudo` so it can access the GPIOs._

**Compiling it yourself:**

You could try to compile it directly on your Raspberry Pi but I highly doubt it will work (maybe on a Pi 4, couldn't try that yet).

To compile for your Raspberry on another machine you will need some dependencies.\
_This from now on assumes that you are on Ubuntu and have the rust toolchain already installed._

- `rustup target add arm-unknown-linux-gnueabihf`
- `sudo apt install libclang-dev gcc-arm-linux-gnueabihf`

To compile run `cargo build --release --target arm-unknown-linux-gnueabihf`.\
The binary should be located at `target/arm-unknown-linux-gnueabihf/release/lighting-manager`.

---

## **Command flags**

```text
Usage: lighting-manager [OPTIONS] --pin <PIN> --count <COUNT>

Options:
  -p, --port <PORT>            Sets the port to listen on [default: 88]
  -a, --address <ADDRESS>      Sets the ip address to listen on [default: 0.0.0.0]
  -P, --pin <PIN>              Sets the pin to which the WS281x LED string is connected
  -c, --count <COUNT>          Sets the count of LEDs in the string
      --log-level <LOG_LEVEL>  Sets the used logging level
                               Possible values: error, warn, info, debug, trace
                               For no logging don't set this option
                               Note: the LOG_LEVEL environment variable overrides this option
  -h, --help                   Print help information
  -V, --version                Print version information
```

---

## **API**

The API listens for HTTP GET requests. Values are parsed from the URI.

### `/`

Returns a message with the current running version of the program.

### `/all_modes`

Returns all currently supported modes of the program as JSON.

**Example:**

Request: `http://your-pi:88/all_modes`\
Response (pretty printed):

```json
{
  "ALARM": 4,
  "COLORRAPE": 5,
  "IDENTIFY": 7,
  "OFF": 0,
  "RAINBOW": 2,
  "SLEEP": 3,
  "STATIC": 1,
  "STROBE": 6
}
```

_note: both the key and the corresponding value can be used to refer to a mode._

### `/mode`

Returns the currently active mode.

**Example:**

Request: `http://your-pi:88/mode`\
Response:

```text
Current mode: STATIC
```

### `/mode/{MODE}`

Change the currently active mode to `{MODE}`.

**Example:**

Request: `http://your-pi:88/mode/RAINBOW`\
Response:

```text
Updated mode: RAINBOW
```

_note: the passed mode is case-insensitive or can also be the corresponding integer (2 in the case of RAINBOW)._

### `/[h,s,v]`

Returns the current value for hue, saturation or value.

**Example:**

Request: `http://your-pi:88/h`\
Response:

```text
Current HUE: 30
```

_note: returned values are floats and range from [0-360) (hue) and [0-1] (saturation & value)._

### `/[h,s,v]/{VALUE}`

Change the current value for hue, saturation or value to `{VALUE}`.

**Example:**

Request: `http://pi-squared.local/h/340.5`\
Response:

```text
Updated HUE: 340.5
```

_note: values get clipped to the previously mentioned ranges._\
_note: for legacy reasons provided values can also be treated as unsigned integers [0-360) (hue) and [0-255] (saturation & value)._

### `/plain/[h,s,v,mode]`

Returns the current value for hue, saturation, value or the currently active mode.

**Example:**

Request: `http://pi-squared.local/plain/h`\
Response:

```text
340.5
```

_note: returned modes are in lowercase._
