<?hh

class AddressValidator {

    static public function isValid(string $addr): bool {

        if (preg_match('/[^1-9A-HJ-NP-Za-km-z]/', $addr)) {
            return false;
        }

        $decoded = self::decodeAddress($addr);

        if (strlen($decoded) != 50) {
            return false;
        }

        if (substr($decoded, 0, 2) != "00" && substr($decoded, 0, 2) != "05") {
            return false;
        }

        $check = substr($decoded, 0, strlen($decoded) - 8);
        $check = pack("H*", $check);
        $check = hash("sha256", $check, true);
        $check = hash("sha256", $check);
        $check = strtoupper($check);
        $check = substr($check, 0, 8);

        return ($check == substr($decoded, strlen($decoded) - 8));
    }

    static protected function decodeAddress(string $data): string {

        $charsetHex = '0123456789ABCDEF';
        $charsetB58 = '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz';

        $raw = "0";
        for ($i = 0; $i < strlen($data); $i++) {
            $current = (string) strpos($charsetB58, $data[$i]);
            $raw = (string) bcmul($raw, "58", 0);
            $raw = (string) bcadd($raw, $current, 0);
        }

        $hex = "";
        while (bccomp($raw, 0) == 1) {
            $dv = (string) bcdiv($raw, "16", 0);
            $rem = (int) bcmod($raw, "16");
            $raw = $dv;
            $hex = $hex . $charsetHex[$rem];
        }

        $withPadding = strrev($hex);
        for ($i = 0; $i < strlen($data) && $data[$i] == "1"; $i++) {
            $withPadding = "00" . $withPadding;
        }

        if (strlen($withPadding) & 1) {
            $withPadding = "0" . $withPadding;
        }

        return $withPadding;
    }

}
