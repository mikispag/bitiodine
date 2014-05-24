<?hh

function write_log(bool $cache_hit, string $request, string $status): void {
	$hm = ($cache_hit) ? 'H' : 'M';
	$line = implode(" ", array(time(), $_SERVER['REMOTE_ADDR'], $hm, $request, $status));
	file_put_contents("/var/log/bitiodine/bitiodine.log", $line . "\n", FILE_APPEND | LOCK_EX);
}
