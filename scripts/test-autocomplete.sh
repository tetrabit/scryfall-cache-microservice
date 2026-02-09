#!/bin/bash
# Test script for autocomplete endpoint

set -e

BASE_URL="${BASE_URL:-http://localhost:8080}"

echo "Testing Autocomplete Endpoint"
echo "=============================="
echo ""

# Test 1: Health check
echo "1. Testing health endpoint..."
response=$(curl -s "${BASE_URL}/health")
if echo "$response" | grep -q "healthy"; then
    echo "✓ Service is healthy"
else
    echo "✗ Service is not healthy"
    exit 1
fi
echo ""

# Test 2: Short query (should return empty)
echo "2. Testing autocomplete with short query (1 char)..."
response=$(curl -s "${BASE_URL}/cards/autocomplete?q=l")
if echo "$response" | grep -q '"data":\[\]'; then
    echo "✓ Short query returns empty results (as expected)"
else
    echo "✗ Expected empty results for short query"
fi
echo ""

# Test 3: "light" prefix
echo "3. Testing autocomplete with 'light' prefix..."
response=$(curl -s "${BASE_URL}/cards/autocomplete?q=light")
echo "$response"
if echo "$response" | grep -q '"object":"catalog"'; then
    echo "✓ Returns catalog object"
    # Check if we got results
    if echo "$response" | grep -q "Lightning"; then
        echo "✓ Found Lightning cards"
    fi
else
    echo "✗ Invalid response format"
fi
echo ""

# Test 4: "sol r" prefix
echo "4. Testing autocomplete with 'sol r' prefix..."
response=$(curl -s "${BASE_URL}/cards/autocomplete?q=sol+r")
echo "$response"
if echo "$response" | grep -q "Sol Ring"; then
    echo "✓ Found 'Sol Ring'"
else
    echo "⚠ 'Sol Ring' not found (might not be in database)"
fi
echo ""

# Test 5: "force" prefix
echo "5. Testing autocomplete with 'force' prefix..."
response=$(curl -s "${BASE_URL}/cards/autocomplete?q=force")
echo "$response"
if echo "$response" | grep -q '"object":"catalog"'; then
    echo "✓ Returns catalog object"
    # Check for common force cards
    if echo "$response" | grep -q "Force of"; then
        echo "✓ Found 'Force of' cards"
    fi
else
    echo "✗ Invalid response format"
fi
echo ""

# Test 6: Case insensitivity
echo "6. Testing case insensitivity..."
response_lower=$(curl -s "${BASE_URL}/cards/autocomplete?q=lightning")
response_upper=$(curl -s "${BASE_URL}/cards/autocomplete?q=LIGHTNING")
response_mixed=$(curl -s "${BASE_URL}/cards/autocomplete?q=LightNing")

if [ "$response_lower" = "$response_upper" ] && [ "$response_lower" = "$response_mixed" ]; then
    echo "✓ Case insensitive search works correctly"
else
    echo "✗ Case sensitivity issue detected"
fi
echo ""

# Test 7: Special characters
echo "7. Testing with special characters..."
response=$(curl -s "${BASE_URL}/cards/autocomplete?q=aether")
echo "$response"
if echo "$response" | grep -q '"object":"catalog"'; then
    echo "✓ Handles special characters"
fi
echo ""

# Test 8: Response time (performance check)
echo "8. Testing response time..."
start_time=$(date +%s%N)
curl -s "${BASE_URL}/cards/autocomplete?q=dragon" > /dev/null
end_time=$(date +%s%N)
duration_ms=$(( (end_time - start_time) / 1000000 ))

if [ $duration_ms -lt 100 ]; then
    echo "✓ Response time: ${duration_ms}ms (target: <100ms)"
elif [ $duration_ms -lt 200 ]; then
    echo "⚠ Response time: ${duration_ms}ms (acceptable, target: <100ms)"
else
    echo "✗ Response time: ${duration_ms}ms (too slow, target: <100ms)"
fi
echo ""

echo "=============================="
echo "Autocomplete tests completed!"
echo ""
echo "Manual testing suggestions:"
echo "  curl \"${BASE_URL}/cards/autocomplete?q=light\""
echo "  curl \"${BASE_URL}/cards/autocomplete?q=sol\""
echo "  curl \"${BASE_URL}/cards/autocomplete?q=force\""
echo "  curl \"${BASE_URL}/cards/autocomplete?q=dragon\""
