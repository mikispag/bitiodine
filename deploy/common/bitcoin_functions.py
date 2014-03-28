import base58
from hashlib import sha256

def hash_to_address(hash):
	vh = "\x00" + hash
	return base58.b58encode(vh + double_sha256(vh)[:4])

def double_sha256(s):
    return sha256(sha256(s).digest()).digest()

def format_satoshis(satoshis):
    if satoshis is None:
        return ''
    if satoshis < 0:
        return '-' + format_satoshis(-satoshis)
    satoshis = int(satoshis)
    integer = satoshis / 10 ** 8
    frac = satoshis % 10 ** 8
    return float(str(integer) +
            ('.' + (('0' * 8) + str(frac))[-8:])
            .rstrip('0').rstrip('.'))

def decode_base58(bc, length):
    CHARSET = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"
    n = 0
    for char in bc:
        n = n * 58 + CHARSET.index(char)
    return n.to_bytes(length, 'big')

def isBTCAddress(address):
    try:
        bcbytes = decode_base58(address, 25)
    except:
        return False
    return bcbytes[-4:] == sha256(sha256(bcbytes[:-4]).digest()).digest()[:4]