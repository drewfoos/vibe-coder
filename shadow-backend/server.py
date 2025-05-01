import os
import base64
import requests # To make requests to OpenAI
from io import BytesIO
from flask import Flask, request, jsonify, Response
from dotenv import load_dotenv
# from PIL import Image # Optional: If you want server-side validation

# Load environment variables from .env file
load_dotenv()

# --- Configuration ---
OPENAI_API_KEY = os.getenv("OPENAI_API_KEY")
OPENAI_API_URL = "https://api.openai.com/v1/chat/completions" # Chat completions endpoint
# Use gpt-4.1 as the default model, still allowing override via env var
OPENAI_MODEL = os.getenv("OPENAI_MODEL", "gpt-4.1") # Changed default to gpt-4.1

# --- Flask App Initialization ---
app = Flask(__name__)

# --- Error Handling ---
class ApiError(Exception):
    """Custom exception for API errors."""
    def __init__(self, message, status_code=500):
        super().__init__(message)
        self.status_code = status_code

@app.errorhandler(ApiError)
def handle_api_error(error):
    response = jsonify({"error": str(error)})
    response.status_code = error.status_code
    return response

@app.errorhandler(Exception)
def handle_generic_error(error):
    # Log the error internally
    app.logger.error(f"An unexpected error occurred: {error}", exc_info=True)
    response = jsonify({"error": "Internal Server Error"})
    response.status_code = 500
    return response

# --- API Endpoint ---
@app.route('/process-image', methods=['POST'])
def process_image():
    """
    Receives Base64 image data, sends it to OpenAI, and returns the response.
    """
    if not OPENAI_API_KEY:
        raise ApiError("OpenAI API key not configured on the server.", 500)

    # 1. Get data from the incoming request
    data = request.get_json()
    if not data:
        raise ApiError("Invalid JSON payload.", 400)

    image_base64 = data.get('image_base64')
    prompt = data.get('prompt', "Describe this image.") # Default prompt

    if not image_base64:
        raise ApiError("Missing 'image_base64' field in JSON payload.", 400)

    # Optional: Basic validation
    try:
        _ = base64.b64decode(image_base64)
    except (base64.binascii.Error, Exception) as e:
         app.logger.warning(f"Received invalid base64 data: {e}")
         raise ApiError(f"Invalid base64 data provided.", 400)

    print(f"Received image data (base64 length: {len(image_base64)}), prompt: '{prompt}'") # Server log

    # 2. Prepare request for OpenAI API
    headers = {
        "Authorization": f"Bearer {OPENAI_API_KEY}",
        "Content-Type": "application/json"
    }

    data_url = f"data:image/png;base64,{image_base64}"

    payload = {
        "model": OPENAI_MODEL, # Uses the configured model (now defaults to gpt-4.1)
        "messages": [
            {
                "role": "user",
                "content": [
                    {"type": "text", "text": prompt},
                    {
                        "type": "image_url",
                        "image_url": {
                            "url": data_url
                            # Optional: Add detail parameter if needed, e.g., "detail": "low" or "high"
                        }
                    }
                ]
            }
        ],
        "max_tokens": 1000 # Increased max_tokens slightly for potentially longer responses
    }

    # 3. Send request to OpenAI
    try:
        print(f"Sending request to OpenAI model: {OPENAI_MODEL}...")
        response = requests.post(OPENAI_API_URL, headers=headers, json=payload, timeout=90) # Increased timeout
        response.raise_for_status() # Raise HTTPError for bad responses (4xx or 5xx)

        openai_data = response.json()
        print("Received response from OpenAI.")

        # 4. Extract the response text
        response_text = openai_data.get('choices', [{}])[0].get('message', {}).get('content', '')

        if not response_text:
             app.logger.warning("OpenAI response missing expected content.")
             raise ApiError("Failed to extract response content from OpenAI.", 500)

        print(f"Extracted response: {response_text[:100]}...")

        # 5. Return the response text to the Rust client
        return Response(response_text, mimetype='text/plain')

    except requests.exceptions.RequestException as e:
        app.logger.error(f"Error communicating with OpenAI: {e}")
        error_details = ""
        if e.response is not None:
            try:
                error_details = e.response.json().get("error", {}).get("message", "")
            except requests.exceptions.JSONDecodeError:
                error_details = e.response.text[:200]
        raise ApiError(f"Failed to communicate with OpenAI. {error_details}", 502)

    except Exception as e:
         app.logger.error(f"Error processing OpenAI response: {e}", exc_info=True)
         raise ApiError("Failed to process OpenAI response.", 500)


# --- Run the App ---
if __name__ == '__main__':
    # Use host='0.0.0.0' to make it accessible on your network for testing
    # Default port is 3000
    app.run(host='0.0.0.0', port=3000, debug=True) # Use debug=False for production
