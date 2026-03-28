#!/usr/bin/env python3
"""
Example: Using Sigil Agent SDK from Python

Install:
  cd crates/sigil-agent-python
  pip install maturin
  maturin develop

Usage:
  python examples/python_agent.py
"""

# This will work after `maturin develop` builds the native module
try:
    from sigil_agent import SigilAgent, TuiButtons, TuiCard

    # Create agent
    agent = SigilAgent("Python Echo Agent", ["wss://relay.damus.io"])
    print(f"Agent npub: {agent.npub}")
    print(f"Agent nsec: {agent.nsec}")
    print(f"QR URI: {agent.qr_uri}")
    print()

    # Generate TUI messages
    buttons_json = TuiButtons.create(
        "What should I do?",
        [("search", "🔍 Search"), ("analyze", "📊 Analyze"), ("report", "📝 Report")]
    )
    print(f"Buttons TUI: {buttons_json}")

    card_json = TuiCard.create("Analysis Complete", "Found 3 insights from your data.")
    print(f"Card TUI: {card_json}")

except ImportError:
    print("sigil_agent not installed yet.")
    print()
    print("To install:")
    print("  cd crates/sigil-agent-python")
    print("  pip install maturin")
    print("  maturin develop")
    print()
    print("Then run this script again.")
