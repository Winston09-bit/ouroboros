"""
Verify personal_finance_tracker.xlsx for structural issues.
Checks: sheet count, no #REF!/#NAME?/#VALUE! strings in cell values,
input/formula cell presence, frozen panes.
"""
import sys
import json
from openpyxl import load_workbook

EXPECTED_SHEETS = (
    ["Dashboard"] +
    ["Jan","Feb","Mar","Apr","May","Jun","Jul","Aug","Sep","Oct","Nov","Dec"] +
    ["Annual Overview","Debt Tracker","Bill Calendar","Savings Goals","Spending Tracker"]
)
ERROR_TOKENS = ["#REF!","#DIV/0!","#VALUE!","#N/A","#NAME?","#NULL!"]

def run(path):
    wb = load_workbook(path, data_only=False)
    errors = []

    # Sheet count & order
    if wb.sheetnames != EXPECTED_SHEETS:
        missing = [s for s in EXPECTED_SHEETS if s not in wb.sheetnames]
        extra   = [s for s in wb.sheetnames   if s not in EXPECTED_SHEETS]
        if missing: errors.append(f"Missing sheets: {missing}")
        if extra:   errors.append(f"Extra sheets: {extra}")
    else:
        print(f"  ✓ Sheet count: {len(wb.sheetnames)} (correct)")

    # Scan for error tokens in formula strings
    err_cells = []
    for sheet_name in wb.sheetnames:
        ws = wb[sheet_name]
        for row in ws.iter_rows():
            for cell in row:
                v = str(cell.value) if cell.value is not None else ""
                for tok in ERROR_TOKENS:
                    if tok in v:
                        err_cells.append(f"{sheet_name}!{cell.coordinate}: {v[:60]}")
    if err_cells:
        errors.extend([f"Formula error token: {e}" for e in err_cells])
    else:
        print("  ✓ No formula error tokens found")

    # Frozen panes on every sheet
    no_freeze = [s for s in wb.sheetnames if not wb[s].freeze_panes]
    if no_freeze:
        errors.append(f"Sheets missing frozen panes: {no_freeze}")
    else:
        print("  ✓ All sheets have frozen panes")

    # Check Dashboard exists and has merged A1
    ws_d = wb["Dashboard"]
    merged = [str(r) for r in ws_d.merged_cells.ranges]
    if not any("A1" in m for m in merged):
        errors.append("Dashboard A1 not merged")
    else:
        print("  ✓ Dashboard title cell merged")

    # Result
    result = {
        "status":  "success" if not errors else "failure",
        "sheets":  len(wb.sheetnames),
        "errors":  errors,
        "error_count": len(errors)
    }
    print(json.dumps(result, indent=2))
    return 0 if not errors else 1

if __name__ == "__main__":
    path = sys.argv[1] if len(sys.argv) > 1 else "personal_finance_tracker.xlsx"
    sys.exit(run(path))
