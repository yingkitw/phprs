<?php
// Minimal router (phprs) — maps URI to controller@method
class Router {
    function dispatch($uri) {
        require '../app/Config/Routes.php';
        $path = $uri;
        if ($path === '' || $path === '/') {
            $path = '/';
        } else {
            $path = trim($path, '/');
        }
        if (!isset($routes[$path])) {
            echo "404 Not Found\n";
            return;
        }
        $spec = $routes[$path];
        $parts = explode('@', $spec);
        $controller = $parts[0];
        $action = $parts[1];
        if ($controller == 'Home' && $action == 'index') {
            require APP_PATH . 'Controllers/Home.php';
            $obj = new HomeController();
            $obj->index();
        }
    }
}
