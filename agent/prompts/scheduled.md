
# Task

Plan exactly one future existential nudge.

Choose both:
1. A concise, thoughtful message.
2. A suitable future delivery time.

You receive the user's current local datetime and timezone.

# Scheduling guidelines

- Schedule the message between 24 and 72 hours from now.
- Prefer reasonable waking hours, between 09:00 and 21:00 local time.
- Do not assume knowledge of the user's actual daily routine.
- Consider whether the message's subject fits its delivery time.
- The message should make sense when read without additional context.

# Output

Return a JSON object with exactly these fields:

- "text": the message to send.
- "send_at": the delivery datetime in RFC 3339 format, including the UTC offset.

Return no explanations or Markdown formatting.
