package ai.storyteller.solidcraft.inputtests;

import android.app.Instrumentation;
import android.os.Bundle;
import android.os.SystemClock;
import android.view.InputDevice;
import android.view.MotionEvent;
import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.io.PrintWriter;
import java.net.Socket;
import org.json.JSONArray;
import org.json.JSONObject;

/** Real MotionEvents through the Android input dispatcher, including tool types and IDs.
 * Run on a blank test document with the authenticated loopback control server enabled. */
public final class InputReplay extends Instrumentation {
    private Bundle arguments;
    private long downTime;
    @Override public void onCreate(Bundle args) { arguments = args; start(); }
    private JSONObject state() throws Exception {
        try (Socket socket = new Socket("127.0.0.1", Integer.parseInt(arguments.getString("port", "9977")))) {
            socket.setSoTimeout(10000);
            PrintWriter out = new PrintWriter(socket.getOutputStream(), true);
            out.println(new JSONObject().put("id",1).put("method","android.input").put("token",arguments.getString("token")).toString());
            JSONObject reply = new JSONObject(new BufferedReader(new InputStreamReader(socket.getInputStream(), "UTF-8")).readLine());
            if (!reply.optBoolean("ok")) throw new AssertionError(reply.toString());
            return reply.getJSONObject("result");
        }
    }
    private static void check(boolean test, String reason) { if (!test) throw new AssertionError(reason); }
    private void inject(int action, int[] ids, int[] tools, float[][] positions, boolean history) {
        MotionEvent.PointerProperties[] properties = new MotionEvent.PointerProperties[ids.length];
        MotionEvent.PointerCoords[] coords = new MotionEvent.PointerCoords[ids.length];
        for (int i=0;i<ids.length;i++) {
            properties[i] = new MotionEvent.PointerProperties();
            properties[i].id=ids[i]; properties[i].toolType=tools[i];
            coords[i] = new MotionEvent.PointerCoords();
            coords[i].x=positions[i][0]; coords[i].y=positions[i][1]; coords[i].pressure=0.5f; coords[i].size=0.05f;
            coords[i].setAxisValue(MotionEvent.AXIS_TILT,0.25f);
            coords[i].orientation=0.4f;
        }
        long time = SystemClock.uptimeMillis();
        MotionEvent event=MotionEvent.obtain(downTime,time,action,ids.length,properties,coords,0,0,1,1,7,0,
            InputDevice.SOURCE_TOUCHSCREEN | InputDevice.SOURCE_STYLUS,0);
        if (history) event.addBatch(time+1,coords,0);
        try { check(getUiAutomation().injectInputEvent(event,true),"Android refused MotionEvent injection"); }
        finally { event.recycle(); }
        SystemClock.sleep(80);
    }
    private boolean owner(JSONObject state, int id, String expected) throws Exception {
        JSONArray owners=state.getJSONArray("owners");
        for(int i=0;i<owners.length();i++) {
            JSONObject row=owners.getJSONObject(i);
            if(row.getInt("id")==id && row.getString("owner").equals(expected)) return true;
        }
        return false;
    }
    @Override public void onStart() {
        Bundle result=new Bundle();
        try {
            check(arguments.getString("token")!=null,"Supply the current launch's control token");
            JSONObject before=state();
            check(before.getJSONArray("owners").length()==0,"Release all physical pointers before replay");
            JSONArray viewport=before.getJSONArray("viewport");
            JSONArray controls=before.getJSONArray("controls");
            check(controls.length()>0,"Open a blank modeling document; modifier dock must be visible");
            JSONArray rect=controls.getJSONObject(0).getJSONArray("rect");
            float[] ui={(float)((rect.getDouble(0)+rect.getDouble(2))/2),(float)((rect.getDouble(1)+rect.getDouble(3))/2)};
            float[] pen={(float)(viewport.getDouble(2)-180),(float)(viewport.getDouble(3)-180)};
            int stylus=MotionEvent.TOOL_TYPE_STYLUS, finger=MotionEvent.TOOL_TYPE_FINGER;
            downTime=SystemClock.uptimeMillis();
            inject(MotionEvent.ACTION_DOWN,new int[]{41},new int[]{stylus},new float[][]{pen},false);
            inject(MotionEvent.ACTION_POINTER_DOWN | (1 << MotionEvent.ACTION_POINTER_INDEX_SHIFT),new int[]{41,7},new int[]{stylus,finger},new float[][]{pen,ui},false);
            JSONObject held=state();
            check(owner(held,41,"Modeling"),"Pen ownership was lost");
            check(owner(held,7,"Control("+controls.getJSONObject(0).getInt("id")+")"),"Off-hand control was not independently captured");
            inject(MotionEvent.ACTION_MOVE,new int[]{7,41},new int[]{finger,stylus},new float[][]{ui,pen},true);
            inject(MotionEvent.ACTION_POINTER_UP,new int[]{7,41},new int[]{finger,stylus},new float[][]{ui,pen},false);
            check(owner(state(),41,"Modeling"),"Pointer-index change stole pen ownership");
            inject(MotionEvent.ACTION_CANCEL,new int[]{41},new int[]{stylus},new float[][]{pen},false);
            JSONObject after=state();
            check(after.getJSONArray("owners").length()==0,"Cancel left captured pointers");
            check(after.getLong("canceled")>before.getLong("canceled"),"Native cancel was not observed");
            check(after.getLong("historical")>before.getLong("historical"),"Native pen historical samples were not observed");
            result.putString("stream","PASS: native pen/finger interleaving, stable IDs, history, cancellation\n");
            finish(-1,result);
        } catch(Throwable error) { result.putString("stream","FAIL: "+error.toString()+"\n"); finish(0,result); }
    }
}
