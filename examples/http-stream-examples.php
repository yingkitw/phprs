<?php
/**
 * HTTP Stream Wrapper Examples
 * 
 * Demonstrates HTTP/HTTPS stream functionality
 */

echo "=== HTTP Stream Wrapper Examples ===\n\n";

// Example 1: Basic HTTP Request
echo "Example 1: Fetch Plain Text\n";
echo "  Note: These examples demonstrate the API\n";
echo "  Actual HTTP requests require network access\n\n";

// Simulated example (would work with real URLs)
$url = "http://example.com/data.txt";
echo "  URL: $url\n";
echo "  Method: file_get_contents()\n";
echo "  Status: Ready to fetch\n\n";

// Example 2: HTTPS Request
echo "Example 2: Secure HTTPS Request\n";
$secure_url = "https://api.example.com/v1/data";
echo "  URL: $secure_url\n";
echo "  Protocol: HTTPS (TLS encrypted)\n";
echo "  Method: file_get_contents()\n\n";

// Example 3: API Data Fetching
echo "Example 3: REST API Integration\n";
echo "  Scenario: Fetch JSON data from API\n";
echo "  Code:\n";
echo "    \$json = file_get_contents('https://api.example.com/users');\n";
echo "    \$data = json_decode(\$json, true);\n";
echo "    foreach (\$data['users'] as \$user) {\n";
echo "      echo \$user['name'];\n";
echo "    }\n\n";

// Example 4: Error Handling
echo "Example 4: HTTP Error Handling\n";
echo "  Code:\n";
echo "    \$content = file_get_contents('http://invalid-url.example');\n";
echo "    if (\$content === false) {\n";
echo "      echo 'Failed to fetch URL';\n";
echo "    }\n\n";

// Example 5: Download File
echo "Example 5: Download Remote File\n";
echo "  Scenario: Download image or document\n";
echo "  Code:\n";
echo "    \$image = file_get_contents('https://example.com/image.jpg');\n";
echo "    file_put_contents('local_image.jpg', \$image);\n\n";

// Example 6: Web Scraping
echo "Example 6: Web Scraping HTML\n";
echo "  Scenario: Extract data from webpage\n";
echo "  Code:\n";
echo "    \$html = file_get_contents('https://example.com');\n";
echo "    preg_match_all('/<title>(.*?)<\/title>/', \$html, \$matches);\n";
echo "    echo \$matches[1][0]; // Page title\n\n";

// Example 7: RSS Feed Reader
echo "Example 7: RSS Feed Parsing\n";
echo "  Scenario: Read RSS/Atom feeds\n";
echo "  Code:\n";
echo "    \$rss = file_get_contents('https://example.com/feed.xml');\n";
echo "    // Parse XML and extract items\n\n";

// Example 8: Check URL Availability
echo "Example 8: URL Availability Check\n";
echo "  Scenario: Verify if URL is accessible\n";
echo "  Code:\n";
echo "    \$content = @file_get_contents('https://example.com');\n";
echo "    \$available = (\$content !== false);\n\n";

// Example 9: Fetch Multiple URLs
echo "Example 9: Batch URL Fetching\n";
echo "  Scenario: Fetch multiple resources\n";
echo "  Code:\n";
echo "    \$urls = ['https://api1.com', 'https://api2.com'];\n";
echo "    foreach (\$urls as \$url) {\n";
echo "      \$data = file_get_contents(\$url);\n";
echo "      // Process data\n";
echo "    }\n\n";

// Example 10: Local vs Remote Files
echo "Example 10: Unified File Access\n";
echo "  Scenario: Same function for local and remote files\n";
echo "  Code:\n";
echo "    // Local file\n";
echo "    \$local = file_get_contents('local_file.txt');\n";
echo "    \n";
echo "    // Remote file\n";
echo "    \$remote = file_get_contents('https://example.com/file.txt');\n\n";

// Practical Example: Weather API
echo "=== Practical Example: Weather API Client ===\n\n";

function fetch_weather($city) {
    echo "Fetching weather for: $city\n";
    // In real implementation:
    // $api_url = "https://api.weather.com/v1/current?city=" . urlencode($city);
    // $json = file_get_contents($api_url);
    // $data = json_decode($json, true);
    // return $data;
    
    echo "  API URL: https://api.weather.com/v1/current?city=$city\n";
    echo "  Method: GET via file_get_contents()\n";
    echo "  Response: JSON data\n";
    return ['temp' => 72, 'condition' => 'Sunny'];
}

$weather = fetch_weather('San Francisco');
echo "  Temperature: " . $weather['temp'] . "°F\n";
echo "  Condition: " . $weather['condition'] . "\n\n";

// Practical Example: GitHub API
echo "=== Practical Example: GitHub Repository Info ===\n\n";

function get_repo_info($owner, $repo) {
    echo "Fetching repository: $owner/$repo\n";
    // In real implementation:
    // $api_url = "https://api.github.com/repos/$owner/$repo";
    // $json = file_get_contents($api_url);
    // $data = json_decode($json, true);
    // return $data;
    
    echo "  API URL: https://api.github.com/repos/$owner/$repo\n";
    echo "  Headers: User-Agent, Accept\n";
    echo "  Response: Repository metadata\n";
    return ['name' => $repo, 'stars' => 1234, 'language' => 'Rust'];
}

$repo = get_repo_info('yingkitw', 'phprs');
echo "  Name: " . $repo['name'] . "\n";
echo "  Stars: " . $repo['stars'] . "\n";
echo "  Language: " . $repo['language'] . "\n\n";

echo "=== HTTP Stream Features ===\n";
echo "✓ HTTP/HTTPS protocol support\n";
echo "✓ Automatic TLS/SSL handling\n";
echo "✓ Seamless integration with file_get_contents()\n";
echo "✓ Error handling for network failures\n";
echo "✓ Support for REST APIs, web scraping, file downloads\n";
echo "✓ Compatible with existing PHP code\n\n";

echo "=== All HTTP Stream Examples Complete ===\n";
