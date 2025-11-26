#!/bin/bash

# Client script to simulate app button click
# This represents Sarah opening the app and clicking on "Tech Talk Live" stream

API_URL="http://localhost:3000"

echo "=== Aava Session Opening Demo ==="
echo ""
echo "Simulating: Sarah opens the app and clicks on 'Tech Talk Live' stream"
echo ""

# # Step 1: Check permissions
# echo "Step 1: Checking permissions..."
# echo "POST $API_URL/api/permissions/check"
# echo ""

# RESPONSE=$(curl -s -X POST "$API_URL/api/permissions/check" \
#   -H "Content-Type: application/json" \
#   -d '{
#     "user_id": "sarah",
#     "stream_id": "tech-talk-live"
#   }')

# echo "Response:"
# echo "$RESPONSE" | jq '.'
# echo ""

# # Check if permission is granted
# HAS_PERMISSION=$(echo "$RESPONSE" | jq -r '.has_permission')

# if [ "$HAS_PERMISSION" != "true" ]; then
#     echo "❌ Permission denied!"
#     exit 1
# fi

# echo "✅ Permission granted!"
# echo ""

# Step 2: Open session
echo "Step 2: Opening session..."
echo "POST $API_URL/api/sessions/open"
echo ""

RESPONSE=$(curl -s -X POST "$API_URL/api/sessions/open" \
  -H "Content-Type: application/json" \
  -d '{
    "viewer_id": "sarah",
    "stream_id": "tech-talk-live"
  }')

echo "Response:"
echo "$RESPONSE" | jq '.'
echo ""

SESSION_ID=$(echo "$RESPONSE" | jq -r '.session_id')
STREAM_URL=$(echo "$RESPONSE" | jq -r '.stream_url')

if [ "$SESSION_ID" != "null" ] && [ "$SESSION_ID" != "" ]; then
    echo "✅ Session opened successfully!"
    echo "   Session ID: $SESSION_ID"
    echo "   Stream URL: $STREAM_URL"
    echo ""
    echo "🎥 Video stream should now be starting..."
else
    echo "❌ Failed to open session!"
    exit 1
fi





