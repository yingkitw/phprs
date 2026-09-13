<?php
// Complex string interpolation demo
// - simple: "$name"
// - simple-syntax accessor: "$arr[key]", "$arr[0]", "$obj->prop"
// - complex (curly): "{$arr['key']}", "{$obj->prop}", "{$obj->method()}"

$user = ["name" => "Alice", "role" => "admin", "tags" => ["php", "rust"]];

echo "name={$user['name']}\n";
echo "role is {$user['role']}, first tag is {$user['tags'][0]}\n";

class Profile {
    public $level = 9;
    public function describe() {
        return "level 9 profile";
    }
}

$profile = new Profile();
echo "profile: {$profile->describe()} (level {$profile->level})\n";

$key = "role";
$nums = [10, 20, 30];
echo "simple syntax: $user[$key] and $nums[0] and $nums[1]\n";

$count = 2;
echo "you have $count message(s), {$user['name']}\n";
