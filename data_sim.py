import socket
import struct
import time
import math

def calc_crc16(data: bytes) -> int:
    crc = 0xFFFF
    for byte in data:
        crc ^= (byte << 8)
        for _ in range(8):
            if crc & 0x8000:
                crc = ((crc << 1) ^ 0x1021) & 0xFFFF
            else:
                crc = (crc << 1) & 0xFFFF
    return crc

sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
target_addr = ("127.0.0.1", 8000)

SYNC_WORD = 0xAA55
TAIL_WORD = 0x55AA
DEVICE_ID = 1
CHANNEL_COUNT = 100

seq = 0
start_time = time.time()
print(f"UDP 8000 전송 시작... (대상: {target_addr})")

while True:
    t = time.time() - start_time
    now_ms = int(time.time() * 1000) % (2**32)

    # 1. 100개 채널 데이터 생성 (IEEE-754 Big-Endian float)
    channels_data = []
    for ch in range(CHANNEL_COUNT):
        val = 10.0 * math.sin(t * 2.0 + ch * 0.1)  # 테스트 정현파
        channels_data.append(val)

    # 2. 헤더 패킹 (Sync: 2B, DevId: 2B, Seq: 4B, Time: 8B, Status: 2B, ChCount: 2B)
    header = struct.pack(">HHQIHH", SYNC_WORD, DEVICE_ID, seq, now_ms, 0, CHANNEL_COUNT)
    payload = struct.pack(f">{CHANNEL_COUNT}f", *channels_data)

    # 3. CRC-16 계산 및 테일 추가
    packet_no_crc = header + payload
    crc = calc_crc16(packet_no_crc)
    footer = struct.pack(">HH", crc, TAIL_WORD)

    full_packet = packet_no_crc + footer
    sock.sendto(full_packet, target_addr)

    seq = (seq + 1) & 0xFFFFFFFF
    time.sleep(0.1) # 100ms 주기 전송