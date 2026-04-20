<?php
/**
 * Regular Expression Examples
 * 
 * Demonstrates all regex functions with practical use cases
 */

echo "=== Regular Expression Examples ===\n\n";

// Example 1: Email Validation
echo "Example 1: Email Validation\n";
$emails = [
    'valid@example.com',
    'user.name@domain.co.uk',
    'invalid@',
    'not-an-email',
    'test@test.com'
];

$email_pattern = '/^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$/';

foreach ($emails as $email) {
    $is_valid = preg_match($email_pattern, $email);
    echo "  $email: " . ($is_valid ? "✓ Valid" : "✗ Invalid") . "\n";
}
echo "\n";

// Example 2: URL Extraction
echo "Example 2: Extract URLs from Text\n";
$text = "Visit https://example.com or http://test.org for more info";
$url_pattern = '/https?:\/\/[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}/';
$count = preg_match_all($url_pattern, $text);
echo "  Text: $text\n";
echo "  URLs found: $count\n\n";

// Example 3: Phone Number Formatting
echo "Example 3: Phone Number Formatting\n";
$phones = ['1234567890', '555-123-4567', '(555) 123-4567'];
foreach ($phones as $phone) {
    // Remove all non-digits
    $cleaned = preg_replace('/\D/', '', $phone);
    // Format as (XXX) XXX-XXXX
    if (strlen($cleaned) === 10) {
        $formatted = preg_replace('/(\d{3})(\d{3})(\d{4})/', '($1) $2-$3', $cleaned);
        echo "  $phone → $formatted\n";
    }
}
echo "\n";

// Example 4: HTML Tag Removal
echo "Example 4: Strip HTML Tags\n";
$html = "<p>Hello <strong>World</strong>!</p>";
$clean = preg_replace('/<[^>]+>/', '', $html);
echo "  Original: $html\n";
echo "  Cleaned: $clean\n\n";

// Example 5: Password Validation
echo "Example 5: Password Strength Validation\n";
$passwords = [
    'weak',
    'Better123',
    'Strong@Pass1',
    'VeryStrong@123'
];

// At least 8 chars, 1 uppercase, 1 lowercase, 1 digit, 1 special char
// (phprs uses the Rust regex engine — no look-ahead; use several checks.)
foreach ($passwords as $pass) {
    $long_enough = strlen($pass) >= 8;
    $has_lower = preg_match('/[a-z]/', $pass);
    $has_upper = preg_match('/[A-Z]/', $pass);
    $has_digit = preg_match('/[0-9]/', $pass);
    $has_special = preg_match('/[@$!%*?&]/', $pass);
    $is_strong = $long_enough && $has_lower && $has_upper && $has_digit && $has_special;
    echo "  $pass: " . ($is_strong ? "✓ Strong" : "✗ Weak") . "\n";
}
echo "\n";

// Example 6: Extract Hashtags
echo "Example 6: Extract Hashtags from Social Media Post\n";
$post = "Loving #PHP and #Rust! #coding #opensource";
$hashtag_pattern = '/#\w+/';
$count = preg_match_all($hashtag_pattern, $post);
echo "  Post: $post\n";
echo "  Hashtags found: $count\n\n";

// Example 7: Date Format Conversion
echo "Example 7: Convert Date Formats\n";
$dates = ['2024-02-23', '2024/12/31', '2024.01.01'];
foreach ($dates as $date) {
    // Convert YYYY-MM-DD to MM/DD/YYYY
    $converted = preg_replace('/(\d{4})[-\/.](\d{2})[-\/.](\d{2})/', '$2/$3/$1', $date);
    echo "  $date → $converted\n";
}
echo "\n";

// Example 8: CSV Parsing
echo "Example 8: Split CSV Line\n";
$csv = "John,Doe,john@example.com,555-1234";
$fields = preg_split('/,/', $csv);
echo "  CSV: $csv\n";
echo "  Fields: " . count($fields) . "\n";
echo "  Name: " . $fields[0] . " " . $fields[1] . "\n\n";

// Example 9: Case-Insensitive Search
echo "Example 9: Case-Insensitive Matching\n";
$text = "The Quick Brown Fox";
$pattern_cs = '/quick/';
$desc_cs = 'Case-sensitive';
$match_cs = preg_match($pattern_cs, $text);
echo "  $desc_cs ($pattern_cs): " . ($match_cs ? "Match" : "No match") . "\n";
$pattern_ci = '/quick/i';
$desc_ci = 'Case-insensitive';
$match_ci = preg_match($pattern_ci, $text);
echo "  $desc_ci ($pattern_ci): " . ($match_ci ? "Match" : "No match") . "\n";
echo "\n";

// Example 10: Word Boundary Matching
echo "Example 10: Whole Word Matching\n";
$text = "The cat and the catalog";
$partial = preg_match('/cat/', $text);
$whole = preg_match('/\bcat\b/', $text);
echo "  Text: $text\n";
echo "  Partial match (/cat/): $partial matches\n";
echo "  Whole word match (/\\bcat\\b/): $whole match\n\n";

// Example 11: Multiline Mode
echo "Example 11: Multiline Pattern Matching\n";
$multiline = "Line 1\nLine 2\nLine 3";
$count = preg_match_all('/^Line/m', $multiline);
echo "  Text: (3 lines)\n";
echo "  Lines starting with 'Line': $count\n\n";

// Example 12: Greedy vs Non-Greedy
echo "Example 12: Greedy vs Non-Greedy Matching\n";
$html = "<div>First</div><div>Second</div>";
$greedy = preg_replace('/<div>.*<\/div>/', '[REPLACED]', $html);
$non_greedy = preg_replace('/<div>.*?<\/div>/', '[REPLACED]', $html);
echo "  Original: $html\n";
echo "  Greedy: $greedy\n";
echo "  Non-greedy: $non_greedy\n\n";

// Example 13: Username Validation
echo "Example 13: Username Validation\n";
$usernames = ['john_doe', 'user123', 'a', 'user@name', 'valid_user_123'];
$username_pattern = '/^[a-zA-Z0-9_]{3,20}$/';
foreach ($usernames as $username) {
    $valid = preg_match($username_pattern, $username);
    echo "  $username: " . ($valid ? "✓ Valid" : "✗ Invalid") . "\n";
}
echo "\n";

// Example 14: Extract Domain from Email
echo "Example 14: Extract Domain from Email\n";
$email = "user@example.com";
$domain = preg_replace('/^[^@]+@/', '', $email);
echo "  Email: $email\n";
echo "  Domain: $domain\n\n";

// Example 15: Remove Extra Whitespace
echo "Example 15: Normalize Whitespace\n";
$text = "Too    many     spaces";
$normalized = preg_replace('/\s+/', ' ', $text);
echo "  Original: '$text'\n";
echo "  Normalized: '$normalized'\n\n";

echo "=== All Regex Examples Complete ===\n";
