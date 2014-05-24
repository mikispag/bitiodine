<?hh

require_once("security.php");

function json_error(int $code, $message): string {
	return json_encode(array("status" => "fail", "data" => null, "code" => $code, "message" => $message), JSON_PRETTY_PRINT);
}

function json_return(array<mixed> $data): string {
	$template = array(
		"status"	=>		"success",
		"data"		=>		$data,
		"code"		=>		200,
		"message"	=>		""
	);
	return json_encode($template, JSON_PRETTY_PRINT);
}

header('Content-Type: application/json');
Security::throttle_ip_api();