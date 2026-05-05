import sys
import os

# Add src to sys.path so we can import loomcli
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "src"))

from loomcli.main import app

if __name__ == "__main__":
    app()
