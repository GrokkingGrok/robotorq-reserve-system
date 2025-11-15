# RoboTorq Printer - Implementation TODO

## 📋 **Document Metadata**
- **Service**: RoboTorq Printer Daemon
- **Version**: v0.1.0 (MVP - Basic Physical RT Printing)
- **Author**: Jonathan Clark (@GrokkingGrok)
- **Date**: November 15, 2025
- **Status**: Planning Phase
- **Platform**: Raspberry Pi / ESP32 (embedded controller)
- **Dependencies**: NATS (network verification), NFC hardware (PN532 module), 3D Printer (Marlin/Klipper firmware)

---

## 💎 **Genesis Print - The First RoboTorq**

### **The Bootstrapping Calculation**

RoboTorq had to start somewhere. Here's the transparent accounting of how the first RT was created:

**The Work**:
- Time invested: 500 hours (Nov 2024 - Nov 2025)
- Developer: Jonathan Clark
- Tools: Claude (Anthropic), GitHub Copilot, Grok

**The RoboNomics**:
- Human cognitive throughput: 6 tokens/sec
- AI nominal throughput: 80 tokens/sec
- Combined productivity: 86 tokens/sec
- Estimated energy per thread: 5 watts
- Total energy equivalent: 215 kWh

**The Calculation**:
```
RoboTorq_genesis = (H_tokens + AI_tokens) × Watts × Hours × torq_factor

Input = 86 tokens/sec × 5W × 500 hrs × 3600 sec/hr
      = 215,000 watt-hours of AI-assisted thinking

Output = RoboTorq Network (estimated value: $250,000)

Torq_factor = $250,000 / 215 kWh = $1,163 per kWh

Genesis RT = 250,000 RT (at $1/RT target price)
```

**The Precedent**:
This establishes the economic model for all future RT:
- Measure energy input (watts × time)
- Measure intelligence multiplier (AI/human token ratio)
- Calculate output value (torq_factor)
- Mint RT proportional to value created

**Market Validation**:
The market immediately validates this calculation via RT/USD exchange rate. If overvalued → RT price drops. If undervalued → RT price rises. Self-correcting.

**The Genesis Story**:
> "I spent 500 hours building RoboTorq with AI assistance.
> 
> My brain: 6 thoughts per second  
> AI's brain: 80 tokens per second  
> Together: 86 tokens/sec of productive work
> 
> Energy: ~5 watts per conversation thread  
> Total: 215 kWh of 'thinking energy'
> 
> Result: A network worth ~$250,000
> 
> Torq factor: $250,000 / 215 kWh = $1,163/kWh
> 
> This means AI-assisted development is 1,163x more valuable per kWh than baseline energy.
> 
> Genesis seed: 250,000 RT (1 RT = $1 target)
> 
> The first RoboTorq was created by measuring the productivity gain from AI assistance - which is exactly what RoboTorq measures going forward!"

---

## 🎯 **Vision: Taking RT Off-Grid**

### **What is "Off-Grid"?**
**Off-grid** = Physical RoboTorq exists outside the digital network
- Not tracked by NATS
- No database record of current location
- Transfers via physical hand-off (like cash)
- Re-enters network via redemption (NFC scan)

**Why Off-Grid Matters**:
- ✅ **Privacy**: Local transactions invisible to network
- ✅ **Resilience**: Works without internet
- ✅ **Accessibility**: Unbanked can participate
- ✅ **Physicality**: Tangible value anchored to recycled materials
- ✅ **Education**: Forces users to understand RT = physical work

---

## 🖨️ **MVP Scope: RoboTorq Printer Daemon**

### **Phase 1 - Core Printing (MVP)**:
**What exists NOW**: Wallet app (burn digital RT), NATS infrastructure
**What's needed**: Physical printer + controller to print bills/coins

**Core Responsibilities**:
1. **Burn Request Reception**: Receive RT burn requests from wallets
2. **Network Verification**: Verify wallet has RT to burn
3. **Serial Number Generation**: Create unique IDs for each bill/coin
4. **NFC Programming**: Write cryptographic data to NFC tags
5. **Print Control**: Pause/resume 3D printer for NFC insertion
6. **Registry Update**: Record serial numbers on-grid (prevent double-redemption)

**Goal**: Users can print physical RT bills from their wallet balance

---

### **Phase 2 - Advanced Features** (Future):
**Additional Features**:
7. **Multi-Denomination Support**: 0.1 RT coins, 1 RT coins, 10 RT bills, 100 RT bills
8. **Material Tracking**: Log recycled materials used (plastic type, weight)
9. **Print Queue Management**: Handle multiple print jobs
10. **Remote Monitoring**: Status updates via NATS
11. **Error Recovery**: Resume interrupted prints
12. **Quality Verification**: Camera-based print inspection

**Goal**: Production-ready printer network

---

### **Phase 3 - Distributed Manufacturing** (Future):
**Additional Features**:
13. **Public Printer Registry**: Find nearby RoboTorq printers
14. **Print-on-Demand Services**: Pay someone to print your RT
15. **Material Marketplace**: Buy/sell recycled filament
16. **Printer Reputation**: Track print quality, reliability
17. **Decentralized Manufacturing**: Community-owned printer network

**Goal**: Anyone can print physical RT locally

---

## 🏗️ **Hardware Architecture**

### **Complete System**:

```
┌─────────────────────────────────────────────────────────────┐
│  3D PRINTER (Modified)                                      │
│  - Ender 3 V2 / Prusa i3 / Bambu Lab P1S                    │
│  - Marlin/Klipper firmware (pause/resume capability)        │
│  - Serial/USB connection to controller                      │
│  - Heated bed (PLA/PETG recycled filament)                  │
└────────────┬────────────────────────────────────────────────┘
             │ Serial (USB or GPIO UART)
             ▼
┌─────────────────────────────────────────────────────────────┐
│  ROBOTORQ PRINTER CONTROLLER                                │
│  Hardware: Raspberry Pi Zero W / ESP32                      │
│                                                             │
│  ┌─────────────────────────────────────┐                   │
│  │  NFC Reader/Writer Module           │                   │
│  │  - PN532 (I2C/SPI)                  │                   │
│  │  - 13.56 MHz (NTAG215 tags)         │                   │
│  │  - Read/Write range: 1-2 inches     │                   │
│  └─────────────────────────────────────┘                   │
│                                                             │
│  ┌─────────────────────────────────────┐                   │
│  │  RoboTorq Printer Daemon (Software) │                   │
│  │  - NATS client (network comm)       │                   │
│  │  - Serial controller (GCODE)        │                   │
│  │  - Crypto library (Ed25519)         │                   │
│  │  - NFC library (PN532 driver)       │                   │
│  └─────────────────────────────────────┘                   │
│                                                             │
│  Storage:                                                   │
│  - Printer private key (signing)                            │
│  - Printer ID (unique identifier)                           │
│  - Print history (local SQLite DB)                          │
└────────────┬────────────────────────────────────────────────┘
             │ Wi-Fi/Ethernet
             ▼
┌─────────────────────────────────────────────────────────────┐
│  ROBOTORQ NETWORK (NATS)                                    │
│  - Burn authorization                                       │
│  - Serial number registry                                   │
│  - Printer registry (authorized printers)                   │
└─────────────────────────────────────────────────────────────┘
```

---

## 📱 **Complete Printing Flow**

### **Step 1: User Initiates Print (Wallet App)**

```
User (Alice):
1. Opens wallet app
2. Balance: 42.5 RT
3. Taps "Print Physical RT"
4. Selects denomination: "10 RT Bill"
5. Selects printer: "My Home Printer"
   - Printer discovered via NFC tap
   - OR entered printer ID manually
   - OR scanned printer QR code
6. Confirms: "Burn 10 RT from wallet"

Wallet App:
- NFC tap printer (transfer print request)
- OR QR code displayed (printer scans)
- OR Wi-Fi broadcast (printer picks up)

Message sent to printer:
{
  "wallet_id": "wallet-alice-abc123",
  "amount_rt": 10.0,
  "denomination": 10,
  "print_request_id": "print-req-789xyz",
  "timestamp": "2025-11-15T14:30:00Z",
  "signature": "0x1234..."  // Signed by wallet's key
}
```

---

### **Step 2: Printer Verifies with Network**

```
Printer Controller receives request:
1. Validates signature (is this really wallet-alice?)
2. Checks printer is authorized
3. Requests burn authorization from network

PUBLISH printer.burn_request {
  "printer_id": "printer-abc123",
  "wallet_id": "wallet-alice-abc123",
  "amount_rt": 10.0,
  "denomination": 10,
  "print_request_id": "print-req-789xyz",
  "requested_at": "2025-11-15T14:30:01Z"
}

Network (Mint Service) checks:
1. Does wallet-alice have 10 RT available?
2. Is printer-abc123 authorized to print?
3. Is print-req-789xyz unique? (prevent double-spend)
4. Is printer under daily limit? (e.g., max 1000 RT/day)

Network response:
PUBLISH printer.burn_authorized.printer-abc123 {
  "print_request_id": "print-req-789xyz",
  "wallet_id": "wallet-alice-abc123",
  "amount_rt": 10.0,
  "denomination": 10,
  "serial_number": "RT-10-2025-ABC123XYZ",  // Globally unique
  "signature": "0x5678...",                 // Network's signature
  "authorized_at": "2025-11-15T14:30:02Z"
}

OR (if rejected):
PUBLISH printer.burn_rejected.printer-abc123 {
  "print_request_id": "print-req-789xyz",
  "reason": "insufficient_balance",
  "details": "Wallet has 8.5 RT, requested 10 RT",
  "rejected_at": "2025-11-15T14:30:02Z"
}
```

---

### **Step 3: Wallet Burns Digital RT (If Authorized)**

```
Network → Wallet:
PUBLISH printer.burn_confirmed.wallet-alice-abc123 {
  "print_request_id": "print-req-789xyz",
  "amount_rt": 10.0,
  "serial_number": "RT-10-2025-ABC123XYZ",
  "printer_id": "printer-abc123",
  "burned_at": "2025-11-15T14:30:02Z"
}

Wallet App:
1. Deducts 10 RT from spendable balance
   - Old balance: 42.5 RT
   - New balance: 32.5 RT

2. Creates transaction record:
   {
     "type": "burn_to_physical",
     "amount_rt": 10.0,
     "serial_number": "RT-10-2025-ABC123XYZ",
     "printer_id": "printer-abc123",
     "timestamp": "2025-11-15T14:30:02Z"
   }

3. UI notification:
   "✅ 10 RT burned! Physical RT printing now..."
   "Serial: RT-10-2025-ABC123XYZ"

RT is now OFF-GRID! 🌐❌ → 💵✅
```

---

### **Step 4: Printer Starts 3D Print (First Half)**

```
Printer Controller:
1. Loads STL file for 10 RT bill
   - Dimensions: 85mm x 60mm x 3mm (similar to credit card)
   - Material: Recycled PLA/PETG filament
   - NFC cavity: 25mm x 15mm x 1mm (center of bill)

2. Sends GCODE to 3D printer:
   G28           ; Home all axes
   G29           ; Auto bed leveling (if supported)
   M104 S210     ; Set hotend temp (PLA)
   M140 S60      ; Set bed temp
   M190 S60      ; Wait for bed temp
   M109 S210     ; Wait for hotend temp
   
   ; Print first 50% (base + NFC cavity)
   G1 X10 Y10 Z0.2 F3000
   G1 E5 F300    ; Extrude
   ; ... (many GCODE lines for bill base) ...
   
   ; Pause at 50% for NFC insertion
   M25           ; Pause print
   M117 Insert NFC tag  ; Display message on printer screen
   M0 S0         ; Wait indefinitely for resume signal

Printer LCD displays:
┌────────────────────────────┐
│  RoboTorq Printer          │
│                            │
│  ⏸️  PAUSED                │
│                            │
│  Insert NFC tag into       │
│  cavity now.               │
│                            │
│  Serial: RT-10-2025-...    │
└────────────────────────────┘
```

---

### **Step 5: NFC Programming & Installation**

```
Printer Controller:
1. Waits for NFC tag detection
   - Continuously polls PN532 reader
   - "Is tag present?"

2. User manually inserts NFC tag (NTAG215)
   - Places tag into printed cavity
   - Tag size: 25mm x 15mm (thin sticker)

3. Tag detected!
   - Controller reads tag UID
   - Verifies tag is blank (not previously used)

4. Program NFC tag with data:
   {
     "protocol_version": "1.0",
     "serial_number": "RT-10-2025-ABC123XYZ",
     "denomination": 10.0,
     "minted_at": "2025-11-15T14:30:10Z",
     "minted_by": "printer-abc123",
     "burned_from_wallet": "wallet-alice-abc123",
     "signature": "0x9abc...",  // Printer's Ed25519 signature
     "public_key": "0xdef0..."  // Printer's public key (for verification)
   }

5. Write to NFC memory:
   - NTAG215 has 540 bytes usable
   - JSON structure: ~200 bytes
   - Remaining space: Reserved for future use

6. Verify write successful:
   - Read back from NFC
   - Compare with original data
   - Verify signature

7. Confirm to network:
   PUBLISH printer.nfc_installed {
     "print_request_id": "print-req-789xyz",
     "serial_number": "RT-10-2025-ABC123XYZ",
     "nfc_uid": "04:A1:B2:C3:D4:E5:F6",
     "verified": true,
     "installed_at": "2025-11-15T14:30:15Z"
   }

Printer LCD updates:
┌────────────────────────────┐
│  ✅ NFC Installed!         │
│                            │
│  Resuming print...         │
│                            │
│  Serial: RT-10-2025-...    │
└────────────────────────────┘
```

---

### **Step 6: Resume Printing (Second Half)**

```
Printer Controller:
1. Sends resume command to 3D printer:
   M24           ; Resume print
   
   ; Print remaining 50% (seal NFC inside)
   ; ... (GCODE continues) ...
   
   ; Print serial number on surface (visible text)
   ; Uses embossed/debossed text in STL
   
   ; Print QR code on back (backup redemption)
   ; QR encodes: serial number + signature
   
   ; Final layers
   G1 Z3.0 F3000
   M104 S0       ; Turn off hotend
   M140 S0       ; Turn off bed
   G28 X0 Y0     ; Home X/Y
   M84           ; Disable steppers
   M117 Print complete!

2. Print completes (~30 minutes total)

Final bill appearance:
┌─────────────────────────────────────┐
│  🤖 ROBOTORQ                        │
│                                     │
│  10 RT                              │
│                                     │
│  "Watts > Wall Street"              │
│                                     │
│  Serial: RT-10-2025-ABC123XYZ       │
│                                     │
│  [QR CODE]                          │
│                                     │
│  (NFC chip embedded inside)         │
└─────────────────────────────────────┘
```

---

### **Step 7: Final Verification & Registry Update**

```
Printer Controller:
1. Final NFC read (through sealed plastic)
   - Verify still readable
   - Verify data integrity
   - Signal strength OK

2. Log print completion:
   - Local database: print_jobs table
   - Record: serial, denomination, wallet, timestamp

3. Notify network:
   PUBLISH printer.print_complete {
     "print_request_id": "print-req-789xyz",
     "serial_number": "RT-10-2025-ABC123XYZ",
     "denomination": 10.0,
     "status": "success",
     "print_duration_seconds": 1847,
     "material_used_grams": 12.5,
     "completed_at": "2025-11-15T15:00:42Z"
   }

Network (Mint Service):
1. Records serial in registry:
   {
     "serial_number": "RT-10-2025-ABC123XYZ",
     "denomination": 10.0,
     "minted_at": "2025-11-15T14:30:10Z",
     "minted_by": "printer-abc123",
     "burned_from_wallet": "wallet-alice-abc123",
     "redeemed": false,
     "redeemed_by": null,
     "redeemed_at": null,
     "status": "off_grid"  // Physical RT in circulation
   }

2. Notify wallet:
   PUBLISH printer.ready_for_pickup.wallet-alice-abc123 {
     "print_request_id": "print-req-789xyz",
     "serial_number": "RT-10-2025-ABC123XYZ",
     "printer_id": "printer-abc123",
     "ready_at": "2025-11-15T15:00:42Z"
   }

Wallet App notification:
"✅ Your 10 RT bill is ready!"
"Serial: RT-10-2025-ABC123XYZ"
"Remove from printer and scan to verify."

Alice removes bill from printer bed.
Physical RT is now in her hands! 💵

RT is OFF-GRID and can be traded locally! 🎉
```

---

## 🔐 **Security & Anti-Counterfeiting**

### **1. Multi-Layer Security**:

```
Layer 1: NFC Cryptographic Signature
- Each bill signed by printer's Ed25519 private key
- Signature verifiable with printer's public key
- Printer keys registered with network

Layer 2: Serial Number Registry
- All serials tracked on-grid (Mint database)
- Redemption check: "Has this serial been redeemed before?"
- Prevents double-spending of physical RT

Layer 3: Printer Authorization
- Only authorized printers can create valid RT
- Printer registration requires wallet stake
- Malicious printers can be blacklisted

Layer 4: Visual Features (Future)
- Holographic stickers (optional)
- UV-reactive ink (embedded in filament)
- Microtext (tiny serial numbers)
- Color-shifting filament

Layer 5: Material Authenticity (Future)
- Recycled material certificates
- Material fingerprinting (spectroscopy)
- Blockchain of recycling provenance
```

---

### **2. NFC Data Structure (NTAG215)**:

```json
{
  "protocol_version": "1.0",
  "serial_number": "RT-10-2025-ABC123XYZ",
  "denomination": 10.0,
  "minted_at": "2025-11-15T14:30:10Z",
  "minted_by": "printer-abc123",
  "burned_from_wallet": "wallet-alice-abc123",
  
  "signature": {
    "algorithm": "Ed25519",
    "value": "0x9abc1234def5678...",
    "signed_fields": ["serial_number", "denomination", "minted_at", "minted_by"]
  },
  
  "printer_public_key": "0xdef0123456789abc...",
  
  "qr_backup": "https://robotorq.network/redeem/RT-10-2025-ABC123XYZ",
  
  "metadata": {
    "material": "recycled_PLA",
    "material_weight_grams": 12.5,
    "co2_locked_grams": 8.3,
    "recycled_source": "plastic_bottles"
  }
}
```

**Memory Usage**:
- Total data: ~350 bytes (JSON)
- NTAG215 capacity: 540 bytes
- Remaining: 190 bytes (future use)

---

### **3. Redemption Verification Flow**:

```
Bob receives physical 10 RT bill from Alice (off-grid trade).
Bob wants to verify it's legit before accepting.

1. Bob opens wallet app
2. Taps "Scan Physical RT"
3. NFC scan of bill

Wallet reads NFC data:
- Serial: RT-10-2025-ABC123XYZ
- Denomination: 10.0 RT
- Signature: 0x9abc...
- Printer: printer-abc123

Wallet verifies:
✅ Step 1: Signature valid?
   - Verify signature with printer's public key
   - Checks: Did printer-abc123 really sign this?

✅ Step 2: Printer authorized?
   - Check network registry: Is printer-abc123 authorized?
   - If blacklisted → REJECT

✅ Step 3: Serial valid?
   - Query network: Does serial exist in registry?
   - Status check: Is it already redeemed?

Network response:
{
  "serial_number": "RT-10-2025-ABC123XYZ",
  "valid": true,
  "redeemed": false,
  "denomination": 10.0,
  "minted_at": "2025-11-15T14:30:10Z"
}

Wallet displays:
┌────────────────────────────┐
│  ✅ VALID ROBOTORQ         │
│                            │
│  Denomination: 10 RT       │
│  Minted: Nov 15, 2025      │
│  Status: Not yet redeemed  │
│                            │
│  [Accept & Redeem]         │
│  [Reject]                  │
└────────────────────────────┘

If Bob taps "Accept & Redeem":
1. Network marks serial as redeemed
2. Bob's wallet credited +10 RT
3. Serial cannot be redeemed again (prevents double-spend)
4. RT is back ON-GRID! 🌐✅

If Bob keeps it off-grid:
- No redemption
- Can hand to Charlie, Diana, etc.
- Stays physical until someone redeems it
```

---

## 🛠️ **Printer Controller Software**

### **Technology Stack**:
- **Language**: Python 3.11+ (Raspberry Pi) OR MicroPython (ESP32)
- **NATS Client**: nats.py (Python) or NATS.ws (WebSocket)
- **NFC Library**: nfcpy (Python) or PN532 driver
- **Serial Control**: pySerial (GCODE communication)
- **Crypto**: PyNaCl (Ed25519 signing)
- **Database**: SQLite (local print history)
- **Config**: TOML or JSON file

---

### **Core Components**:

#### **1. PrinterDaemon** (Main Service)
**Responsibility**: Orchestrate entire printing process

```python
class PrinterDaemon:
    def __init__(self, config):
        self.printer_id = config.printer_id
        self.nats_client = NATSClient(config.nats_url)
        self.nfc_controller = NFCController()
        self.printer_controller = GCodeController(config.serial_port)
        self.crypto = CryptoManager(config.private_key_path)
        self.db = PrinterDatabase(config.db_path)
        
    async def start(self):
        # Subscribe to burn requests
        await self.nats_client.subscribe(
            f"printer.burn_authorized.{self.printer_id}",
            self.handle_burn_authorized
        )
        
        # Subscribe to wallet requests (NFC/QR)
        await self.nats_client.subscribe(
            "printer.print_request",
            self.handle_print_request
        )
        
        # Start heartbeat (announce printer online)
        asyncio.create_task(self.heartbeat())
        
    async def handle_burn_authorized(self, msg):
        data = json.loads(msg.data)
        
        # Start print job
        job = PrintJob(
            request_id=data["print_request_id"],
            serial=data["serial_number"],
            denomination=data["denomination"],
            wallet_id=data["wallet_id"]
        )
        
        await self.execute_print_job(job)
        
    async def execute_print_job(self, job):
        # Step 1: Load STL and start print
        await self.printer_controller.start_print(job.denomination)
        
        # Step 2: Wait for pause (50% mark)
        await self.printer_controller.wait_for_pause()
        
        # Step 3: Program NFC
        nfc_data = self.create_nfc_data(job)
        await self.nfc_controller.write_tag(nfc_data)
        await self.nfc_controller.verify_tag(nfc_data)
        
        # Step 4: Resume print
        await self.printer_controller.resume_print()
        
        # Step 5: Wait for completion
        await self.printer_controller.wait_for_complete()
        
        # Step 6: Final verification
        await self.nfc_controller.verify_readable()
        
        # Step 7: Notify network
        await self.nats_client.publish("printer.print_complete", {
            "print_request_id": job.request_id,
            "serial_number": job.serial,
            "status": "success",
            "completed_at": datetime.utcnow().isoformat()
        })
        
        # Step 8: Save to database
        await self.db.save_print_job(job)
```

---

#### **2. NFCController**
**Responsibility**: Read/write NFC tags

```python
class NFCController:
    def __init__(self):
        self.clf = nfc.ContactlessFrontend('usb')  # PN532 via USB
        
    async def write_tag(self, data: dict) -> bool:
        """Write JSON data to NTAG215 tag"""
        tag = await self.wait_for_tag()
        
        if tag.product != "NTAG215":
            raise ValueError("Wrong tag type - need NTAG215")
        
        # Convert data to bytes
        json_bytes = json.dumps(data).encode('utf-8')
        
        # Write to NDEF message
        ndef_message = ndef.Message(ndef.Record("application/json", data=json_bytes))
        tag.ndef.message = ndef_message
        
        return True
        
    async def verify_tag(self, expected_data: dict) -> bool:
        """Read tag and verify data matches"""
        tag = await self.wait_for_tag()
        
        # Read NDEF message
        ndef_message = tag.ndef.message
        json_bytes = ndef_message[0].data
        actual_data = json.loads(json_bytes.decode('utf-8'))
        
        # Compare
        if actual_data["serial_number"] != expected_data["serial_number"]:
            raise ValueError("Serial number mismatch!")
        
        if actual_data["signature"] != expected_data["signature"]:
            raise ValueError("Signature mismatch!")
        
        return True
        
    async def wait_for_tag(self, timeout=60):
        """Wait for NFC tag to be placed on reader"""
        start = time.time()
        while time.time() - start < timeout:
            tag = self.clf.connect(rdwr={'on-connect': lambda tag: False})
            if tag:
                return tag
            await asyncio.sleep(0.5)
        raise TimeoutError("No NFC tag detected")
```

---

#### **3. GCodeController**
**Responsibility**: Control 3D printer via serial

```python
class GCodeController:
    def __init__(self, serial_port: str, baudrate: int = 115200):
        self.serial = serial.Serial(serial_port, baudrate, timeout=1)
        self.paused = False
        self.complete = False
        
    async def start_print(self, denomination: float):
        """Start printing bill/coin based on denomination"""
        stl_file = self.get_stl_file(denomination)
        gcode_file = self.slice_stl(stl_file)
        
        # Send GCODE line by line
        with open(gcode_file, 'r') as f:
            for line in f:
                await self.send_gcode(line.strip())
                
    async def send_gcode(self, command: str):
        """Send GCODE command to printer"""
        self.serial.write(f"{command}\n".encode())
        
        # Wait for "ok" response
        while True:
            response = self.serial.readline().decode().strip()
            if response == "ok":
                break
            elif "M25" in response or "M0" in response:
                self.paused = True
                break
                
    async def wait_for_pause(self):
        """Wait until printer pauses (M25 or M0)"""
        while not self.paused:
            await asyncio.sleep(0.1)
        print("Printer paused - ready for NFC insertion")
        
    async def resume_print(self):
        """Send M24 (resume) to printer"""
        await self.send_gcode("M24")
        self.paused = False
        print("Print resumed")
        
    async def wait_for_complete(self):
        """Wait until print finishes"""
        # Monitor serial for completion messages
        while not self.complete:
            response = self.serial.readline().decode().strip()
            if "Print complete" in response or "M84" in response:
                self.complete = True
            await asyncio.sleep(0.1)
        print("Print complete!")
        
    def get_stl_file(self, denomination: float) -> str:
        """Get STL file path for denomination"""
        stl_map = {
            0.1: "models/coin_0.1RT.stl",
            1.0: "models/coin_1RT.stl",
            10.0: "models/bill_10RT.stl",
            100.0: "models/bill_100RT.stl"
        }
        return stl_map.get(denomination)
        
    def slice_stl(self, stl_file: str) -> str:
        """Convert STL to GCODE (via slicer)"""
        # Call PrusaSlicer or Cura CLI
        gcode_file = stl_file.replace(".stl", ".gcode")
        
        # Simplified - in reality, call slicer with pause inserted
        os.system(f"prusa-slicer --export-gcode {stl_file} --output {gcode_file}")
        
        return gcode_file
```

---

#### **4. CryptoManager**
**Responsibility**: Sign NFC data with printer's key

```python
class CryptoManager:
    def __init__(self, private_key_path: str):
        with open(private_key_path, 'rb') as f:
            self.private_key = nacl.signing.SigningKey(f.read())
        self.public_key = self.private_key.verify_key
        
    def sign_nfc_data(self, data: dict) -> dict:
        """Add signature to NFC data"""
        # Create message to sign
        message = f"{data['serial_number']}{data['denomination']}{data['minted_at']}{data['minted_by']}"
        message_bytes = message.encode('utf-8')
        
        # Sign
        signed = self.private_key.sign(message_bytes)
        signature_hex = signed.signature.hex()
        
        # Add signature to data
        data["signature"] = {
            "algorithm": "Ed25519",
            "value": signature_hex,
            "signed_fields": ["serial_number", "denomination", "minted_at", "minted_by"]
        }
        
        data["printer_public_key"] = self.public_key.encode().hex()
        
        return data
        
    def verify_signature(self, data: dict) -> bool:
        """Verify signature on NFC data"""
        # Reconstruct message
        message = f"{data['serial_number']}{data['denomination']}{data['minted_at']}{data['minted_by']}"
        message_bytes = message.encode('utf-8')
        
        # Get signature
        signature_hex = data["signature"]["value"]
        signature_bytes = bytes.fromhex(signature_hex)
        
        # Get public key
        public_key_hex = data["printer_public_key"]
        verify_key = nacl.signing.VerifyKey(bytes.fromhex(public_key_hex))
        
        # Verify
        try:
            verify_key.verify(message_bytes, signature_bytes)
            return True
        except nacl.exceptions.BadSignatureError:
            return False
```

---

#### **5. PrinterDatabase**
**Responsibility**: Local SQLite storage

```sql
-- Database schema

CREATE TABLE print_jobs (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    print_request_id    TEXT UNIQUE NOT NULL,
    serial_number       TEXT UNIQUE NOT NULL,
    denomination        REAL NOT NULL,
    wallet_id           TEXT NOT NULL,
    status              TEXT NOT NULL,  -- 'pending', 'printing', 'paused', 'complete', 'failed'
    started_at          TIMESTAMP,
    paused_at           TIMESTAMP,
    resumed_at          TIMESTAMP,
    completed_at        TIMESTAMP,
    material_used_grams REAL,
    print_duration_sec  INTEGER,
    error_message       TEXT
);

CREATE TABLE nfc_tags (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    print_job_id    INTEGER REFERENCES print_jobs(id),
    nfc_uid         TEXT UNIQUE NOT NULL,
    serial_number   TEXT NOT NULL,
    programmed_at   TIMESTAMP NOT NULL,
    verified_at     TIMESTAMP
);

CREATE TABLE printer_config (
    key     TEXT PRIMARY KEY,
    value   TEXT NOT NULL
);

-- Indexes
CREATE INDEX idx_print_jobs_serial ON print_jobs(serial_number);
CREATE INDEX idx_print_jobs_wallet ON print_jobs(wallet_id);
CREATE INDEX idx_nfc_tags_serial ON nfc_tags(serial_number);
```

---

## 📡 **NATS Topics**

### **Subscribed Topics** (Printer → Network):

```typescript
// 1. Burn authorization (from Mint)
SUBSCRIBE printer.burn_authorized.{printer_id} {
  "print_request_id": string,
  "wallet_id": string,
  "amount_rt": number,
  "denomination": number,
  "serial_number": string,
  "signature": string,
  "authorized_at": timestamp
}

// 2. Burn rejection (from Mint)
SUBSCRIBE printer.burn_rejected.{printer_id} {
  "print_request_id": string,
  "reason": string,
  "details": string,
  "rejected_at": timestamp
}

// 3. Printer commands (admin control - future)
SUBSCRIBE printer.command.{printer_id} {
  "command": "pause" | "resume" | "cancel" | "status",
  "issued_by": string,
  "issued_at": timestamp
}
```

---

### **Published Topics** (Network → Printer):

```typescript
// 1. Burn request (from printer to Mint)
PUBLISH printer.burn_request {
  "printer_id": string,
  "wallet_id": string,
  "amount_rt": number,
  "denomination": number,
  "print_request_id": string,
  "requested_at": timestamp
}

// 2. NFC installed
PUBLISH printer.nfc_installed {
  "print_request_id": string,
  "serial_number": string,
  "nfc_uid": string,
  "verified": boolean,
  "installed_at": timestamp
}

// 3. Print complete
PUBLISH printer.print_complete {
  "print_request_id": string,
  "serial_number": string,
  "denomination": number,
  "status": "success" | "failed",
  "print_duration_seconds": number,
  "material_used_grams": number,
  "completed_at": timestamp
}

// 4. Print failed
PUBLISH printer.print_failed {
  "print_request_id": string,
  "serial_number": string,
  "error": string,
  "failed_at": timestamp
}

// 5. Printer heartbeat (status updates)
PUBLISH printer.heartbeat.{printer_id} {
  "printer_id": string,
  "status": "online" | "printing" | "paused" | "offline",
  "current_job": string | null,
  "jobs_completed_today": number,
  "rt_printed_today": number,
  "timestamp": timestamp
}
```

---

## 🔧 **Printer Registration**

### **Initial Setup Flow**:

```
1. User buys/builds RoboTorq Printer
   - Raspberry Pi + PN532 NFC module
   - 3D printer (Ender 3, Prusa, etc.)
   - Installs RoboTorq Printer Daemon software

2. Generate printer keypair:
   $ robotorq-printer keygen
   
   Output:
   ✅ Keypair generated!
   Private key: /home/pi/.robotorq/printer_key.pem (KEEP SECRET!)
   Public key: 0xabc123def456...
   Printer ID: printer-2025-xyz789

3. User opens wallet app
   - Taps "Register Printer"
   - Scans printer QR code (contains public key + printer ID)
   
   OR
   
   - Enters printer ID manually
   - Enters public key

4. Wallet stakes RT (e.g., 100 RT)
   - Anti-spam mechanism
   - Malicious printers lose stake if blacklisted
   
5. Wallet publishes registration:
   PUBLISH printer.register {
     "printer_id": "printer-2025-xyz789",
     "public_key": "0xabc123def456...",
     "owner_wallet": "wallet-alice-abc123",
     "stake_amount": 100.0,
     "model": "Ender 3 V2",
     "location": "San Francisco, CA",  // Optional
     "max_daily_rt": 1000.0,            // Self-imposed limit
     "registered_at": "2025-11-15T10:00:00Z"
   }

6. Network (Mint) verifies:
   - Wallet has 100 RT to stake
   - Printer ID is unique
   - Public key format valid
   
7. Network approves:
   PUBLISH printer.registered {
     "printer_id": "printer-2025-xyz789",
     "status": "authorized",
     "stake_locked": 100.0,
     "daily_limit_rt": 1000.0,
     "authorized_at": "2025-11-15T10:00:01Z"
   }

8. Printer daemon receives confirmation:
   - Saves authorization to local DB
   - Starts listening for print requests
   - Begins heartbeat broadcasts

Printer is now LIVE! 🖨️✅
```

---

## 🗄️ **Network Registry (Mint Service Extension)**

### **Database Schema (Add to Mint)**:

```sql
-- Authorized printers
CREATE TABLE printers (
    id                  VARCHAR(64) PRIMARY KEY,  -- "printer-2025-xyz789"
    public_key          TEXT NOT NULL UNIQUE,
    owner_wallet_id     VARCHAR(64) NOT NULL,
    stake_amount        FLOAT NOT NULL,
    status              VARCHAR(20) NOT NULL DEFAULT 'authorized',  -- 'authorized', 'suspended', 'blacklisted'
    daily_limit_rt      FLOAT NOT NULL DEFAULT 1000.0,
    model               VARCHAR(100),
    location            VARCHAR(255),
    registered_at       TIMESTAMP NOT NULL DEFAULT NOW(),
    last_heartbeat_at   TIMESTAMP
);

-- Physical RT serial numbers
CREATE TABLE physical_rt_registry (
    serial_number       VARCHAR(64) PRIMARY KEY,  -- "RT-10-2025-ABC123XYZ"
    denomination        FLOAT NOT NULL,
    minted_at           TIMESTAMP NOT NULL,
    minted_by           VARCHAR(64) NOT NULL REFERENCES printers(id),
    burned_from_wallet  VARCHAR(64) NOT NULL,
    
    -- Redemption tracking
    redeemed            BOOLEAN NOT NULL DEFAULT false,
    redeemed_by         VARCHAR(64),
    redeemed_at         TIMESTAMP,
    
    -- Status
    status              VARCHAR(20) NOT NULL DEFAULT 'off_grid',  -- 'off_grid', 'redeemed', 'invalidated'
    
    -- Material tracking (future)
    material_type       VARCHAR(50),
    material_weight_g   FLOAT,
    co2_locked_g        FLOAT,
    
    FOREIGN KEY (redeemed_by) REFERENCES wallets(id)
);

-- Print history (for analytics)
CREATE TABLE print_jobs (
    id                  VARCHAR(64) PRIMARY KEY,
    printer_id          VARCHAR(64) NOT NULL REFERENCES printers(id),
    wallet_id           VARCHAR(64) NOT NULL,
    serial_number       VARCHAR(64) NOT NULL REFERENCES physical_rt_registry(serial_number),
    denomination        FLOAT NOT NULL,
    status              VARCHAR(20) NOT NULL,  -- 'pending', 'printing', 'complete', 'failed'
    started_at          TIMESTAMP,
    completed_at        TIMESTAMP,
    print_duration_sec  INTEGER,
    material_used_g     FLOAT
);

CREATE INDEX idx_printers_owner ON printers(owner_wallet_id);
CREATE INDEX idx_printers_status ON printers(status);
CREATE INDEX idx_physical_rt_serial ON physical_rt_registry(serial_number);
CREATE INDEX idx_physical_rt_redeemed ON physical_rt_registry(redeemed);
CREATE INDEX idx_print_jobs_printer ON print_jobs(printer_id);
CREATE INDEX idx_print_jobs_wallet ON print_jobs(wallet_id);
```

---

## 🧪 **Testing Strategy**

### **Hardware Testing** (Integration):
```python
# test_nfc_controller.py
async def test_nfc_write_read():
    nfc = NFCController()
    
    test_data = {
        "serial_number": "RT-10-TEST-000001",
        "denomination": 10.0,
        "minted_at": datetime.utcnow().isoformat(),
        "minted_by": "printer-test-001"
    }
    
    # Write
    await nfc.write_tag(test_data)
    
    # Read back
    read_data = await nfc.read_tag()
    
    assert read_data["serial_number"] == test_data["serial_number"]
    assert read_data["denomination"] == test_data["denomination"]

# test_gcode_controller.py
async def test_print_pause_resume():
    gcode = GCodeController("/dev/ttyUSB0")
    
    # Start print
    await gcode.start_print(10.0)
    
    # Wait for pause
    await gcode.wait_for_pause()
    assert gcode.paused == True
    
    # Resume
    await gcode.resume_print()
    assert gcode.paused == False
```

---

### **End-to-End Test Script** (PowerShell):

```powershell
# test-e2e-printer.ps1

Write-Host "=== RoboTorq Printer E2E Test ===" -ForegroundColor Green

# Step 1: Start printer daemon
Write-Host "`n[1] Starting printer daemon..." -ForegroundColor Yellow
Start-Process python -ArgumentList "printer_daemon.py" -NoNewWindow

Start-Sleep -Seconds 3

# Step 2: Check printer registered
Write-Host "[2] Checking printer registration..." -ForegroundColor Yellow
$printers = Invoke-RestMethod -Uri "http://localhost:8084/printers"

if ($printers.Count -eq 0) {
    Write-Host "❌ No printers registered" -ForegroundColor Red
    exit 1
}

Write-Host "✅ Printer registered: $($printers[0].id)" -ForegroundColor Green

# Step 3: Simulate wallet burn request
Write-Host "[3] Simulating print request..." -ForegroundColor Yellow

$printRequest = @{
    wallet_id = "wallet-test-alice"
    amount_rt = 10.0
    denomination = 10
    print_request_id = "print-req-test-001"
} | ConvertTo-Json

Invoke-RestMethod -Uri "http://localhost:8084/print" `
    -Method Post `
    -ContentType "application/json" `
    -Body $printRequest

# Step 4: Monitor print status
Write-Host "[4] Monitoring print job..." -ForegroundColor Yellow

$timeout = 60
$elapsed = 0

while ($elapsed -lt $timeout) {
    $status = Invoke-RestMethod -Uri "http://localhost:8084/status"
    
    Write-Host "  Status: $($status.current_status)" -ForegroundColor Cyan
    
    if ($status.current_status -eq "paused") {
        Write-Host "  ⏸️  Paused for NFC insertion" -ForegroundColor Yellow
        Write-Host "  (Simulating NFC tag placement...)" -ForegroundColor Yellow
        
        # Simulate NFC tag detected
        Invoke-RestMethod -Uri "http://localhost:8084/nfc/detected" -Method Post
        break
    }
    
    Start-Sleep -Seconds 2
    $elapsed += 2
}

# Step 5: Wait for completion
Write-Host "[5] Waiting for print completion..." -ForegroundColor Yellow

while ($elapsed -lt 300) {  # 5 min timeout
    $status = Invoke-RestMethod -Uri "http://localhost:8084/status"
    
    if ($status.current_status -eq "complete") {
        Write-Host "✅ Print complete!" -ForegroundColor Green
        Write-Host "  Serial: $($status.serial_number)" -ForegroundColor Cyan
        break
    }
    
    Start-Sleep -Seconds 5
    $elapsed += 5
}

Write-Host "`n=== Test Complete ===" -ForegroundColor Green
```

---

## 📋 **Implementation Checklist**

### **Phase 1: Hardware Setup** (Week 1)
- [ ] Acquire hardware components:
  - [ ] Raspberry Pi Zero W or ESP32
  - [ ] PN532 NFC module (I2C or SPI)
  - [ ] NTAG215 NFC tags (pack of 100)
  - [ ] USB cable for printer connection
  - [ ] Power supply
- [ ] Connect NFC module to Pi (I2C pins)
- [ ] Connect Pi to 3D printer (USB serial)
- [ ] Install Raspberry Pi OS
- [ ] Install Python dependencies (nfcpy, pyserial, PyNaCl, etc.)

### **Phase 2: Software Development** (Week 2-3)
- [ ] Implement NFCController
  - [ ] write_tag() - Write JSON to NTAG215
  - [ ] read_tag() - Read and parse JSON
  - [ ] verify_tag() - Verify signature
  - [ ] wait_for_tag() - Polling loop
- [ ] Implement GCodeController
  - [ ] send_gcode() - Serial communication
  - [ ] start_print() - Begin print job
  - [ ] wait_for_pause() - Detect M25/M0
  - [ ] resume_print() - Send M24
  - [ ] wait_for_complete() - Monitor completion
- [ ] Implement CryptoManager
  - [ ] sign_nfc_data() - Ed25519 signing
  - [ ] verify_signature() - Signature verification
  - [ ] generate_keypair() - One-time setup
- [ ] Implement PrinterDatabase
  - [ ] SQLite schema creation
  - [ ] save_print_job()
  - [ ] query_print_history()

### **Phase 3: NATS Integration** (Week 3)
- [ ] Implement NATSClient
  - [ ] Connect to network
  - [ ] Subscribe to burn_authorized
  - [ ] Publish burn_request
  - [ ] Publish print_complete
  - [ ] Heartbeat broadcasts
- [ ] Test offline mode (queue requests)
- [ ] Test reconnection logic

### **Phase 4: 3D Models** (Week 4)
- [ ] Design STL models:
  - [ ] 0.1 RT coin (25mm diameter)
  - [ ] 1 RT coin (30mm diameter)
  - [ ] 10 RT bill (85mm x 60mm)
  - [ ] 100 RT bill (100mm x 70mm)
- [ ] Add NFC cavity to models
- [ ] Add embossed serial number
- [ ] Add QR code surface feature
- [ ] Slice with pause at 50% layer

### **Phase 5: Testing & Iteration** (Week 5)
- [ ] Unit tests (all components)
- [ ] Integration tests (hardware + software)
- [ ] End-to-end test (full print flow)
- [ ] Stress test (10 prints in a row)
- [ ] Error recovery tests (power loss, NFC failure, etc.)

### **Phase 6: Network Integration** (Week 6)
- [ ] Extend Mint service:
  - [ ] Add printer registration endpoint
  - [ ] Add burn authorization logic
  - [ ] Add physical RT registry table
  - [ ] Add redemption verification endpoint
- [ ] Update wallet app:
  - [ ] Add "Print Physical RT" button
  - [ ] Add NFC scanning for redemption
  - [ ] Add physical RT transaction logging

### **Phase 7: Documentation & Deployment** (Week 7)
- [ ] Write setup guide (hardware assembly)
- [ ] Write installation guide (software)
- [ ] Write user manual (wallet → printer flow)
- [ ] Create video tutorial
- [ ] Publish on GitHub
- [ ] Deploy test printer (alpha users)

---

## 🚀 **Future Enhancements**

### **Phase 2: Advanced Features**
- [ ] Multi-material printing (color-coded denominations)
- [ ] UV-reactive security features
- [ ] Holographic sticker integration
- [ ] Auto-slicing (STL → GCODE on device)
- [ ] Remote monitoring dashboard
- [ ] Print queue management
- [ ] Material usage tracking
- [ ] CO2 sequestration calculation

### **Phase 3: Public Printer Network**
- [ ] Printer discovery (nearby printers)
- [ ] Print-as-a-Service (pay someone to print your RT)
- [ ] Printer reputation system
- [ ] Material marketplace (buy/sell recycled filament)
- [ ] Decentralized manufacturing incentives
- [ ] Community-owned printer DAO

---

## 📚 **Resources**

### **Hardware**:
- [PN532 NFC Module Datasheet](https://www.nxp.com/docs/en/nxp/data-sheets/PN532_C1.pdf)
- [NTAG215 Datasheet](https://www.nxp.com/docs/en/data-sheet/NTAG213_215_216.pdf)
- [Raspberry Pi GPIO Pinout](https://pinout.xyz/)

### **Software**:
- [nfcpy Documentation](https://nfcpy.readthedocs.io/)
- [PySerial Documentation](https://pythonhosted.org/pyserial/)
- [PyNaCl (Ed25519) Documentation](https://pynacl.readthedocs.io/)
- [Marlin Firmware GCODE Reference](https://marlinfw.org/meta/gcode/)

### **3D Printing**:
- [PrusaSlicer](https://www.prusa3d.com/page/prusaslicer_424/)
- [Cura](https://ultimaker.com/software/ultimaker-cura)
- [Thingiverse](https://www.thingiverse.com/) (3D model repository)

---

## ✅ **Definition of Done**

- [ ] Printer daemon runs on Raspberry Pi
- [ ] NFC tags successfully programmed
- [ ] 3D printer pauses/resumes via daemon control
- [ ] Complete print-to-redemption flow working
- [ ] Network integration (NATS pub/sub)
- [ ] Signature verification passing
- [ ] Anti-counterfeiting measures in place
- [ ] User documentation complete
- [ ] Alpha test with 10 users successful

---

**Last Updated**: November 15, 2025  
**Author**: Jonathan Clark  
**Status**: Ready for Hardware Acquisition  
**Estimated Timeline**: 7 weeks to MVP

---

## 🎯 **Key Terminology**

**RoboTorq Terminology** (NOT Crypto!):
- ✅ **"Off-grid"**: Physical RT not tracked by network
- ✅ **"On-grid"**: Digital RT tracked via NATS
- ✅ **"Printing"**: Creating physical RT (not minting!)
- ✅ **"Burning"**: Taking digital RT off-grid
- ✅ **"Redeeming"**: Bringing physical RT back on-grid
- ✅ **"Network"**: RoboTorq NATS infrastructure
- ✅ **"Registry"**: Database of serial numbers

**We are NOT**:
- ❌ Blockchain
- ❌ Cryptocurrency
- ❌ "On-chain" / "Off-chain"
- ❌ "Minting" (we're PRINTING!)

**We ARE**:
- ✅ Encrypted physical currency
- ✅ NATS-based message network
- ✅ Recycled material value storage
- ✅ Local, offline-capable cash

---

**END OF PRINTER IMPLEMENTATION TODO**
