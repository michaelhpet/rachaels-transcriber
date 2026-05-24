# Custom hook for webrtcvad (webrtcvad-wheels on Windows).
# The contrib hook's copy_metadata('webrtcvad') crashes because
# the distribution is named 'webrtcvad-wheels', not 'webrtcvad'.
# This no-op hook replaces it, relying on --hidden-import for the module.
