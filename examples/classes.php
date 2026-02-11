<?php
// Classes Example
// Demonstrates PHP class definitions and usage

class Person {
    private $name;
    private $age;

    public function __construct($name, $age) {
        $this->name = $name;
        $this->age = $age;
    }

    public function getName() {
        return $this->name;
    }

    public function getAge() {
        return $this->age;
    }

    public function greet() {
        return "Hello, I'm {$this->name} and I'm {$this->age} years old.";
    }
}

// Create instance
$person = new Person("PHP-RS", 1);
echo $person->greet() . "\n";

// Inheritance
class Developer extends Person {
    private $language;

    public function __construct($name, $age, $language) {
        parent::__construct($name, $age);
        $this->language = $language;
    }

    public function getLanguage() {
        return $this->language;
    }

    public function greet() {
        return parent::greet() . " I code in {$this->language}.";
    }
}

$dev = new Developer("PHP-RS", 1, "Rust");
echo $dev->greet() . "\n";

