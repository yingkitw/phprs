<?php
// Attributes Example
// Demonstrates PHP 8.0 attributes

#[Route('/api/users')]
function getUsers() {
    return 'user list';
}

echo getUsers();
echo "\n";

#[Entity]
class Product {
    #[Column]
    public function name() {
        return 'Widget';
    }
}

$p = new Product();
echo $p->name();
echo "\n";
