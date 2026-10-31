<?php
// Interface (implements) and class-hierarchy demo.
// Covered by tests/interface_implements.rs and the examples matrix.

// --- 1. A simple interface contract ---------------------------------------
interface Greeter {
    public function greet($name);
}

class EnglishGreeter implements Greeter {
    public function greet($name) {
        return "Hello, $name!";
    }
}

$g = new EnglishGreeter();
echo $g->greet("World"), "\n";

// --- 2. Multiple interfaces ------------------------------------------------
interface JsonSerializable2 {
    public function toJson();
}
interface Identifiable {
    public function id();
}

class User implements Identifiable, JsonSerializable2 {
    public $id;
    public $name;

    public function __construct($id, $name) {
        $this->id = $id;
        $this->name = $name;
    }
    public function id() {
        return $this->id;
    }
    public function toJson() {
        return '{"id": ' . $this->id . ', "name": "' . $this->name . '"}';
    }
}

$u = new User(1, "Ada");
echo $u->id(), " -> ", $u->toJson(), "\n";

// --- 3. Interface default implementation ----------------------------------
// A method with a body on the interface provides a default; the class may
// inherit it or override it.
interface Describable {
    public function describe() {
        return "generic thing";
    }
}

class Box implements Describable { }
class Crate implements Describable {
    public function describe() {
        return "wooden crate";
    }
}

$box = new Box();
echo $box->describe(), "\n";
$crate = new Crate();
echo $crate->describe(), "\n";

// --- 4. interface extends interface ---------------------------------------
interface Animal {
    public function speak();
}
interface Dog extends Animal {
    public function fetch();
}

class Labrador implements Dog {
    public function speak() {
        return "woof";
    }
    public function fetch() {
        return "ball";
    }
}

$d = new Labrador();
echo $d->speak(), "/", $d->fetch(), "\n";

// --- 5. Interface constants -------------------------------------------------
interface ColorPalette {
    const PRIMARY = "blue";
}

class Theme implements ColorPalette { }
echo Theme::PRIMARY, "\n";

// --- 6. extends + implements, inherited members ----------------------------
class BaseWidget {
    public $label = "widget";
    public function render() {
        return "render: {$this->label}";
    }
}

interface Clickable {
    public function onClick();
}

class Button extends BaseWidget implements Clickable {
    public function onClick() {
        return "clicked {$this->label}";
    }
}

$b = new Button();
echo $b->render(), " / ", $b->onClick(), "\n";

// --- 7. Contract enforcement (compile-time) --------------------------------
// Uncommenting the class below fails to compile, as in PHP:
//
// class Broken implements Greeter { }
//   -> Class Broken contains 1 abstract method and must therefore be declared
//      abstract or implement the remaining methods (Greeter::greet)

echo "interfaces OK\n";
