<?php
// Loop control: break / continue with explicit levels.
// Also demonstrates the for-loop rule that `continue` still runs the increment.

$odds = 0;
for ($i = 1; $i <= 10; $i++) {
    if ($i % 2 == 0) {
        continue; // skip evens; the increment ($i++) still runs
    }
    if ($i > 7) {
        break; // stop once we pass 7
    }
    $odds += $i;
}
echo "odds sum: $odds\n"; // 1 + 3 + 5 + 7 = 16

// break 2 / continue 2 across nested loops.
$grid = "";
for ($row = 0; $row < 4; $row++) {
    for ($col = 0; $col < 4; $col++) {
        if ($col > $row) {
            continue 2; // next row once the triangle edge is passed
        }
        if ($row == 3 && $col == 2) {
            break 2; // stop the whole scan late in the last row
        }
        $grid .= "($row,$col) ";
    }
}
echo "grid: $grid\n";

// while(true) + break as a search loop.
$haystack = array(3, 8, 15, 4, 27);
$found = -1;
$idx = 0;
foreach ($haystack as $h) {
    if ($h > 10) {
        $found = $idx;
        break;
    }
    $idx++;
}
echo "first >10 at index $found (value {$haystack[$found]})\n";
