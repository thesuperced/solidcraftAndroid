    // App-specific input configuration adapter, retained by sync-artcraft-templates.sh.
    public int cadInputConfig(int item) {
        if (item == 0) return android.view.ViewConfiguration.get(this).getScaledTouchSlop();
        if (item == 1) return android.view.ViewConfiguration.getLongPressTimeout();
        return android.view.ViewConfiguration.getDoubleTapTimeout();
    }
