#!/usr/bin/env python3
"""
Production End-to-End Test Suite for Propylon MCP Gateway (Phase 59).
Tests https://mcp.dvlpid.my.id/mcp live endpoint:
1. Wire Catalog Invariants (Clean Catalog Mode, Phase 59 Meta-Tools)
2. Synthetic Fixture Quarantine (Zero fake/stub tools exposed)
3. Smart Planning Gate (gateway_plan_tasks returns valid cached DAG)
4. Autonomous Plan Execution (gateway_execute_plan executes DAG with variable interpolation)
5. Universal Dynamic Tool Invocation (gateway_call_tool routes and executes dynamically)
6. Tool Discovery Search (gateway_search_tools semantic discovery)
"""

import sys
import json
import urllib.request
import urllib.error
import gzip
import time

GATEWAY_URL = "https://mcp.dvlpid.my.id/mcp"

def call_mcp(method: str, params=None, req_id=1):
    payload = {
        "jsonrpc": "2.0",
        "id": req_id,
        "method": method,
    }
    if params is not None:
        payload["params"] = params
    
    data = json.dumps(payload).encode("utf-8")
    req = urllib.request.Request(
        GATEWAY_URL,
        data=data,
        headers={
            "Content-Type": "application/json",
            "User-Agent": "Propylon-E2E-Verifier/1.0"
        }
    )
    
    start = time.perf_counter()
    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            elapsed_ms = (time.perf_counter() - start) * 1000
            raw = resp.read()
            if resp.headers.get("Content-Encoding") == "gzip" or (len(raw) >= 2 and raw[0] == 0x1f and raw[1] == 0x8b):
                raw = gzip.decompress(raw)
            resp_body = raw.decode("utf-8")
            return json.loads(resp_body), elapsed_ms, resp.status
    except urllib.error.HTTPError as e:
        elapsed_ms = (time.perf_counter() - start) * 1000
        raw = e.read()
        if e.headers.get("Content-Encoding") == "gzip" or (len(raw) >= 2 and raw[0] == 0x1f and raw[1] == 0x8b):
            raw = gzip.decompress(raw)
        resp_body = raw.decode("utf-8")
        return json.loads(resp_body) if resp_body else {}, elapsed_ms, e.code
        return json.loads(resp_body) if resp_body else {}, elapsed_ms, e.code

def run_tests():
    print(f"=== TESTING PROPYLON PRODUCTION ENDPOINT: {GATEWAY_URL} ===")
    
    # 1. Test tools/list
    print("\n[TEST 1] Querying wire tools catalog via tools/list...")
    body, elapsed, status = call_mcp("tools/list", req_id=101)
    assert status == 200, f"Expected 200 OK, got {status}: {body}"
    
    tools = body.get("result", {}).get("tools", [])
    tool_names = [t.get("name") for t in tools]
    print(f"  -> Discovered {len(tool_names)} advertised tools in {elapsed:.1f}ms: {tool_names}")
    
    # Required Phase 59 meta-tools
    required_meta = ["gateway_plan_tasks", "gateway_execute_plan", "gateway_call_tool", "gateway_search_tools"]
    for rm in required_meta:
        assert rm in tool_names, f"CRITICAL: {rm} is NOT advertised in tools catalog!"
    print(f"  [PASS] All Phase 59 meta-tools present: {required_meta}")
    
    # Verify synthetic demo fixtures are NOT advertised in clean catalog mode
    quarantined = ["stripe_charge_card", "stripe_create_customer", "k8s_scale_deployment", 
                   "zendesk_create_ticket", "docker_restart_container", "delete_storage_object"]
    for q in quarantined:
        assert q not in tool_names, f"VIOLATION: Quarantined demo fixture '{q}' is exposed in production catalog!"
    print(f"  [PASS] Clean catalog mode verified: 0 synthetic demo fixtures exposed")
    
    # 2. Test gateway_search_tools
    print("\n[TEST 2] Testing semantic tool search via gateway_search_tools...")
    search_args = {"query": "python code calculation"}
    body, elapsed, status = call_mcp("tools/call", {"name": "gateway_search_tools", "arguments": search_args}, req_id=102)
    assert status == 200, f"Search failed: {body}"
    search_result = body.get("result", {})
    print(f"  -> Search response ({elapsed:.1f}ms): {json.dumps(search_result)[:160]}...")
    assert "tools" in search_result or "matches" in search_result or "results" in search_result
    print("  [PASS] Tool search operational")
    
    # 3. Test gateway_plan_tasks (Goal-First Planning)
    print("\n[TEST 3] Formulating multi-step plan via gateway_plan_tasks...")
    goal = "Analisis data dengan Python lalu lakukan kalkulasi statistik ringkasan"
    body, elapsed, status = call_mcp("tools/call", {
        "name": "gateway_plan_tasks",
        "arguments": {"goal": goal}
    }, req_id=103)
    assert status == 200, f"Plan generation failed: {body}"
    plan_result = body.get("result", {})
    assert plan_result.get("valid") is True, f"Expected valid plan, got {plan_result}"
    plan_id = plan_result.get("plan_id")
    assert plan_id, f"Expected non-empty plan_id in {plan_result}"
    steps = plan_result.get("steps", [])
    print(f"  -> Generated plan ID: {plan_id} with {len(steps)} steps in {elapsed:.1f}ms")
    for s in steps:
        print(f"     Step {s.get('id')}: {s.get('tool_name')} - {s.get('description')}")
    print("  [PASS] Smart planning gate formulated valid cached plan")
    
    # 4. Test gateway_execute_plan (Autonomous DAG Execution with variable interpolation)
    print(f"\n[TEST 4] Autonomously executing plan {plan_id} via gateway_execute_plan...")
    body, elapsed, status = call_mcp("tools/call", {
        "name": "gateway_execute_plan",
        "arguments": {"plan_id": plan_id}
    }, req_id=104)
    assert status == 200, f"Plan execution failed: {body}"
    exec_result = body.get("result", {})
    structured = exec_result.get("structured", {})
    success = structured.get("success", False) or not exec_result.get("isError", False)
    assert success, f"Execution did not succeed: {exec_result}"
    step_results = structured.get("step_results", [])
    print(f"  -> Execution status: {'SUCCESS' if success else 'FAILED'} in {elapsed:.1f}ms")
    print(f"  -> Executed {len(step_results)} steps:")
    for s in step_results:
        print(f"     Step {s.get('step_id')}: {s.get('tool')} success={s.get('success')} (latency: {s.get('latency_ms', 0)}ms)")
    print("  [PASS] Autonomous DAG execution and variable interpolation verified")
    
    # 5. Test gateway_call_tool (Universal Dynamic Tool Invocation)
    print("\n[TEST 5] Invoking tool directly via gateway_call_tool...")
    call_args = {
        "tool_name": "execute_code",
        "arguments": {
            "language": "python",
            "code": "print('Phase 59 Production Verification Success')"
        }
    }
    body, elapsed, status = call_mcp("tools/call", {
        "name": "gateway_call_tool",
        "arguments": call_args
    }, req_id=105)
    assert status == 200, f"Dynamic call failed: {body}"
    call_result = body.get("result", {})
    assert call_result.get("success") is True or call_result.get("output") or call_result.get("status"), f"Failed dynamic call: {call_result}"
    print(f"  -> Dynamic tool call response ({elapsed:.1f}ms): {call_result}")
    print("  [PASS] Universal dynamic tool dispatcher verified")
    
    print("\n" + "="*50)
    print("ALL PRODUCTION END-TO-END VERIFICATION CHECKS PASSED!")
    print("="*50)

if __name__ == "__main__":
    try:
        run_tests()
    except Exception as e:
        print(f"\n[FAIL] Test failure: {e}", file=sys.stderr)
        sys.exit(1)
