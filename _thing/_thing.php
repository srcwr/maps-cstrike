<?php

// (Invoke-WebRequest -Uri http://mofu.mofumofu.site.nfoservers.com/_thing.php -Body (Get-Childitem -file *.bsp | select length, name | ConvertTo-Json) -Method POST -UseBasicParsing).Content

header('Content-Type: text/plain');

echo "Comparing against maps from https://github.com/srcwr/maps-cstrike\n\n";

$hadany = false;
$everything = json_decode(file_get_contents("_thing.json"), true);
$json = json_decode(file_get_contents('php://input'), true, 3, JSON_THROW_ON_ERROR);

foreach($json as $data) {
	$name = preg_replace('/\.bsp$/', '', strtolower($data['Name']));
	$length = intval($data['Length']);
	if (!isset($everything[$name]) || !in_array($length, $everything[$name])) {
		$hadany = true;
		echo "UNIQUE! $name\n";
	}
}

if (!$hadany) {
	echo "No unique filename&filesize combinations!\n";
}
