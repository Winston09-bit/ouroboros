"""
Ultimate Budget Tracker v4 — ultimate_budget_v4.xlsx
No charts (add manually). Strict row tracking. Clean Calibri/sage/terra palette.
"""

import calendar
from openpyxl import Workbook
from openpyxl.styles import Font, PatternFill, Alignment, Border, Side
from openpyxl.utils import get_column_letter
from openpyxl.worksheet.datavalidation import DataValidation
from openpyxl.formatting.rule import FormulaRule

# ─── PALETTE ────────────────────────────────────────────────────────────────
SAGE_DARK   = "3D5C3A"
SAGE_MID    = "5C8A52"
SAGE_LIGHT  = "C8DDB8"
SAGE_PALE   = "EEF4E8"
TERRA       = "B05E3A"
TERRA_LIGHT = "EDD5C5"
CREAM       = "FAF7F2"
GOLD        = "BFA06A"
WHITE       = "FFFFFF"
CHARCOAL    = "2A2A2A"
INPUT_BLUE  = "1A4FA0"
GREEN       = "1E8449"
RED         = "B03A2E"
GRAY_DIS    = "EEEEEE"
BORDER_GRAY = "DCDCDC"

# ─── ROW HEIGHTS ─────────────────────────────────────────────────────────────
ROW_TITLE    = 48
ROW_SUBTITLE = 20
ROW_COLHDR   = 22
ROW_SECTION  = 22
ROW_DATA     = 18
ROW_SUBTOTAL = 20
ROW_GRAND    = 24
ROW_NET      = 26

# ─── NUMBER FORMATS ──────────────────────────────────────────────────────────
MONEY = '#,##0;[RED]-#,##0;"-"'
PCT   = '0.0%;[RED]-0.0%;"-"'
DATE_FMT = 'DD/MM/YYYY'

# ─── MONTHS ──────────────────────────────────────────────────────────────────
MO_SHORT = ["Jan","Feb","Mar","Apr","May","Jun","Jul","Aug","Sep","Oct","Nov","Dec"]
MO_LONG  = ["January","February","March","April","May","June",
            "July","August","September","October","November","December"]

CATS = [
    ("HOUSING",               ["Rent/Mortgage","Utilities","Electricity","Internet","Home Insurance"]),
    ("TRANSPORT",             ["Car Payment","Fuel","Car Insurance","Parking","Public Transport"]),
    ("FOOD & DINING",         ["Groceries","Restaurants","Coffee","Food Delivery","Work Lunch"]),
    ("HEALTH & WELLNESS",     ["Health Insurance","Gym","Pharmacy","Doctor","Dental"]),
    ("LIFESTYLE",             ["Streaming Services","Entertainment","Hobbies","Personal Care","Books"]),
    ("PERSONAL",              ["Clothing","Gifts","Education","Pet Care","Miscellaneous"]),
    ("SAVINGS & INVESTMENTS", ["Emergency Fund","Pension/IRA","Stocks","Vacation Fund","Other Savings"]),
    ("DEBT PAYMENTS",         ["Credit Card 1","Credit Card 2","Student Loan","Car Loan","Other"]),
]

# ─── STYLE BUILDERS ──────────────────────────────────────────────────────────
def fl(h):  return PatternFill("solid", fgColor=h)
def fn(size=9, bold=False, color=CHARCOAL, italic=False):
    return Font(name="Calibri", size=size, bold=bold, color=color, italic=italic)
def al(h="left", v="center", wrap=False, indent=0):
    return Alignment(horizontal=h, vertical=v, wrap_text=wrap, indent=indent)

THIN  = Side(style="thin",   color=BORDER_GRAY)
THICK = Side(style="medium", color=BORDER_GRAY)
GOLD_THICK = Side(style="medium", color=GOLD)
SAGE_THICK = Side(style="medium", color=SAGE_MID)

def thin_border():
    return Border(left=THIN, right=THIN, top=THIN, bottom=THIN)

def gold_bottom_border():
    return Border(left=THIN, right=THIN, top=THIN, bottom=GOLD_THICK)

def section_bottom_border():
    return Border(left=THIN, right=THIN, top=THIN, bottom=SAGE_THICK)

def outer_border(color=SAGE_MID):
    s = Side(style="medium", color=color)
    return Border(left=s, right=s, top=s, bottom=s)

def no_gridlines(ws): ws.sheet_view.showGridLines = False

def rh(ws, row, h): ws.row_dimensions[row].height = h

def sc(ws, row, col, value="", bold=False, size=9, color=CHARCOAL,
       bg=WHITE, h_align="left", fmt=None, italic=False, border=None, wrap=False, indent=0):
    """Set cell with full styling."""
    c = ws.cell(row=row, column=col, value=value)
    c.font      = fn(size, bold, color, italic)
    c.fill      = fl(bg)
    c.alignment = al(h_align, "center", wrap, indent)
    c.border    = border if border else thin_border()
    if fmt: c.number_format = fmt
    return c

def mc(ws, r1, c1, r2, c2, value="", bold=False, size=9, color=CHARCOAL,
       bg=WHITE, h_align="center", fmt=None, italic=False, border=None, wrap=False, indent=0):
    """Merge + set cell."""
    ws.merge_cells(start_row=r1, start_column=c1, end_row=r2, end_column=c2)
    c = ws.cell(row=r1, column=c1, value=value)
    c.font      = fn(size, bold, color, italic)
    c.fill      = fl(bg)
    c.alignment = al(h_align, "center", wrap, indent)
    c.border    = border if border else thin_border()
    if fmt: c.number_format = fmt
    return c

def inp(ws, row, col, value=0, fmt=MONEY, bg=WHITE):
    return sc(ws, row, col, value, False, 9, INPUT_BLUE, bg, "right", fmt)

def frm(ws, row, col, formula, fmt=MONEY, color=CHARCOAL, bg=WHITE):
    return sc(ws, row, col, formula, False, 9, color, bg, "right", fmt)

def fill_merged_neighbors(ws, row, c_start, c_end, bg):
    """Fill merged-range sibling cells (they become MergedCell objects)."""
    for col in range(c_start, c_end + 1):
        cell = ws.cell(row=row, column=col)
        try:
            cell.fill   = fl(bg)
            cell.border = thin_border()
        except AttributeError:
            pass  # MergedCell — skip

def set_outer_border_range(ws, r1, c1, r2, c2, color=SAGE_MID):
    """Draw medium border around a rectangular range."""
    s = Side(style="medium", color=color)
    for row in range(r1, r2 + 1):
        for col in range(c1, c2 + 1):
            cell = ws.cell(row=row, column=col)
            left   = s if col == c1 else THIN
            right  = s if col == c2 else THIN
            top    = s if row == r1 else THIN
            bottom = s if row == r2 else THIN
            try:
                cell.border = Border(left=left, right=right, top=top, bottom=bottom)
            except AttributeError:
                pass

# ═══════════════════════════════════════════════════════════════════════════
# WORKBOOK
# ═══════════════════════════════════════════════════════════════════════════
wb = Workbook()
ws_dash = wb.active
ws_dash.title = "Dashboard"

# Build monthly sheets first so we have row tracker for Dashboard formulas
rows = {}   # rows[mo_short][key] = row_number

# ═══════════════════════════════════════════════════════════════════════════
# MONTHLY SHEETS (Jan–Dec)
# ═══════════════════════════════════════════════════════════════════════════
for mo, m_long in zip(MO_SHORT, MO_LONG):
    ws = wb.create_sheet(title=mo)
    no_gridlines(ws)
    rows[mo] = {}

    # ── Row 1: Title ─────────────────────────────────────────────────────
    mc(ws,1,1,1,7, f"{m_long.upper()}  ·  MONTHLY BUDGET",
       True,18,WHITE,SAGE_DARK,"center")
    rh(ws,1,ROW_TITLE)
    rows[mo]["title"] = 1

    # ── Row 2: Subtitle ──────────────────────────────────────────────────
    mc(ws,2,1,2,7, f"Track income and expenses for {m_long}  ·  Blue cells = your input",
       False,9,WHITE,GOLD,"left",italic=True)
    rh(ws,2,ROW_SUBTITLE)
    rows[mo]["subtitle"] = 2

    # ── Row 3: Column headers ─────────────────────────────────────────────
    for ci,h in enumerate(["CATEGORY","SUBCATEGORY","BUDGETED","ACTUAL","DIFFERENCE","% USED","NOTES"]):
        sc(ws,3,ci+1,h,True,9,WHITE,SAGE_DARK,"center")
    rh(ws,3,ROW_COLHDR)
    rows[mo]["col_headers"] = 3

    r = 4  # current row pointer

    # ── Row 4: INCOME section header ─────────────────────────────────────
    mc(ws,r,1,r,7,"INCOME",True,10,WHITE,TERRA,"left",indent=1)
    rh(ws,r,ROW_SECTION)
    rows[mo]["income_section"] = r
    r += 1

    # ── Rows 5–7: Income items ────────────────────────────────────────────
    rows[mo]["income_start"] = r
    inc_items = ["Primary Income","Side Income","Other Income"]
    for ji,item in enumerate(inc_items):
        bg = SAGE_PALE if ji%2==0 else WHITE
        sc(ws,r,1,"INCOME",    bg=bg, h_align="left")
        sc(ws,r,2,item,        bg=bg, h_align="left")
        inp(ws,r,3, bg=bg)
        inp(ws,r,4, bg=bg)
        frm(ws,r,5,f"=C{r}-D{r}",     MONEY,CHARCOAL,bg)
        frm(ws,r,6,f"=IFERROR(D{r}/C{r},0)", PCT,CHARCOAL,bg)
        sc(ws,r,7,"",bg=bg)
        rh(ws,r,ROW_DATA)
        r += 1
    rows[mo]["income_end"] = r - 1

    # ── Row 8: TOTAL INCOME ───────────────────────────────────────────────
    rows[mo]["income_total"] = r
    ti = r
    mc(ws,r,1,r,2,"TOTAL INCOME",True,9,CHARCOAL,SAGE_LIGHT,"left",
       border=section_bottom_border())
    fill_merged_neighbors(ws,r,1,2,SAGE_LIGHT)
    frm(ws,r,3,f"=SUM(C{rows[mo]['income_start']}:C{rows[mo]['income_end']})",MONEY,CHARCOAL,SAGE_LIGHT)
    frm(ws,r,4,f"=SUM(D{rows[mo]['income_start']}:D{rows[mo]['income_end']})",MONEY,CHARCOAL,SAGE_LIGHT)
    frm(ws,r,5,f"=C{r}-D{r}",MONEY,CHARCOAL,SAGE_LIGHT)
    frm(ws,r,6,f"=IFERROR(D{r}/C{r},0)",PCT,CHARCOAL,SAGE_LIGHT)
    sc(ws,r,7,"",bg=SAGE_LIGHT)
    rh(ws,r,ROW_SUBTOTAL)
    r += 1

    # ── Expense categories ────────────────────────────────────────────────
    sub_refs_c = []
    sub_refs_d = []

    for cat_name, sub_items in CATS:
        cat_key = cat_name.replace(" & ","_").replace(" ","_")

        # Section header
        mc(ws,r,1,r,7,cat_name,True,10,WHITE,SAGE_MID,"left",indent=1)
        rh(ws,r,ROW_SECTION)
        rows[mo][f"{cat_name}_section"] = r
        r += 1

        cat_start = r
        rows[mo][f"{cat_name}_start"] = r
        for ji,sub in enumerate(sub_items):
            bg = SAGE_PALE if ji%2==0 else WHITE
            sc(ws,r,1,cat_name,bg=bg,h_align="left")
            sc(ws,r,2,sub,     bg=bg,h_align="left")
            inp(ws,r,3, bg=bg)
            inp(ws,r,4, bg=bg)
            frm(ws,r,5,f"=C{r}-D{r}",         MONEY,CHARCOAL,bg)
            frm(ws,r,6,f"=IFERROR(D{r}/C{r},0)",PCT,CHARCOAL,bg)
            sc(ws,r,7,"",bg=bg)
            rh(ws,r,ROW_DATA)
            r += 1
        cat_end = r - 1
        rows[mo][f"{cat_name}_end"] = cat_end

        # Subtotal row
        sub_r = r
        rows[mo][f"{cat_name}_total"] = sub_r
        mc(ws,sub_r,1,sub_r,2,f"Subtotal — {cat_name}",
           True,9,CHARCOAL,SAGE_LIGHT,"left",border=section_bottom_border())
        fill_merged_neighbors(ws,sub_r,1,2,SAGE_LIGHT)
        frm(ws,sub_r,3,f"=SUM(C{cat_start}:C{cat_end})",MONEY,CHARCOAL,SAGE_LIGHT)
        frm(ws,sub_r,4,f"=SUM(D{cat_start}:D{cat_end})",MONEY,CHARCOAL,SAGE_LIGHT)
        frm(ws,sub_r,5,f"=C{sub_r}-D{sub_r}",MONEY,CHARCOAL,SAGE_LIGHT)
        frm(ws,sub_r,6,f"=IFERROR(D{sub_r}/C{sub_r},0)",PCT,CHARCOAL,SAGE_LIGHT)
        sc(ws,sub_r,7,"",bg=SAGE_LIGHT)
        rh(ws,sub_r,ROW_SUBTOTAL)
        sub_refs_c.append(f"C{sub_r}")
        sub_refs_d.append(f"D{sub_r}")
        r += 1

    # ── Grand Total Expenses ──────────────────────────────────────────────
    gt = r
    rows[mo]["grand_total"] = gt
    mc(ws,gt,1,gt,2,"GRAND TOTAL EXPENSES",True,10,WHITE,TERRA,"left")
    fill_merged_neighbors(ws,gt,1,2,TERRA)
    frm(ws,gt,3,"="+"+".join(sub_refs_c),MONEY,WHITE,TERRA)
    frm(ws,gt,4,"="+"+".join(sub_refs_d),MONEY,WHITE,TERRA)
    frm(ws,gt,5,f"=C{gt}-D{gt}",MONEY,WHITE,TERRA)
    sc(ws,gt,6,"",bg=TERRA)
    sc(ws,gt,7,"",bg=TERRA)
    rh(ws,gt,ROW_GRAND)
    r += 1

    # ── Net Cash Flow ─────────────────────────────────────────────────────
    ncf = r
    rows[mo]["net_flow"] = ncf
    mc(ws,ncf,1,ncf,2,"NET CASH FLOW",True,11,WHITE,SAGE_DARK,"left")
    fill_merged_neighbors(ws,ncf,1,2,SAGE_DARK)
    frm(ws,ncf,3,f"=C{ti}-C{gt}",MONEY,WHITE,SAGE_DARK)
    frm(ws,ncf,4,f"=D{ti}-D{gt}",MONEY,WHITE,SAGE_DARK)
    frm(ws,ncf,5,f"=C{ncf}-D{ncf}",MONEY,WHITE,SAGE_DARK)
    sc(ws,ncf,6,"",bg=SAGE_DARK)
    sc(ws,ncf,7,"",bg=SAGE_DARK)
    rh(ws,ncf,ROW_NET)

    ws.freeze_panes = "A4"
    ws.sheet_properties.tabColor = SAGE_MID
    ws.column_dimensions["A"].width = 24
    ws.column_dimensions["B"].width = 22
    for ci,w in enumerate([14,14,14,11,20],3):
        ws.column_dimensions[get_column_letter(ci)].width = w


# ═══════════════════════════════════════════════════════════════════════════
# DASHBOARD
# ═══════════════════════════════════════════════════════════════════════════
no_gridlines(ws_dash)
ws_dash.sheet_properties.tabColor = SAGE_DARK

# YTD savings formula (reused in KPI cards + budget summary)
sav_cat = "SAVINGS & INVESTMENTS"
ytd_parts = [f"{m}!D{rows[m][f'{sav_cat}_total']}" for m in MO_SHORT]
YTD_F = "=IFERROR("+"+".join(ytd_parts)+",0)"

jan_gt  = rows["Jan"]["grand_total"]
jan_ti  = rows["Jan"]["income_total"]

# ── Row 1: Title ──────────────────────────────────────────────────────────
mc(ws_dash,1,1,1,9,"PERSONAL FINANCE COMMAND CENTER  ·  2026",
   True,18,WHITE,SAGE_DARK,"center")
rh(ws_dash,1,ROW_TITLE)

# ── Row 2: Subtitle ───────────────────────────────────────────────────────
mc(ws_dash,2,1,2,9,
   "Live Summary  ·  All figures update automatically from monthly sheets",
   False,9,WHITE,GOLD,"left",italic=True)
rh(ws_dash,2,ROW_SUBTITLE)

# ── Rows 3–6: KPI CARDS ───────────────────────────────────────────────────
# Layout: A–C card1 | D spacer | E–G card2 | H spacer | I extra
# Row 3=label, Row 4=value  /  Row 5=label, Row 6=value
rh(ws_dash,3,20); rh(ws_dash,4,30); rh(ws_dash,5,20); rh(ws_dash,6,30)

# Spacer cols
for r_ in range(3,7):
    ws_dash.cell(row=r_,column=4).fill  = fl(CREAM)
    ws_dash.cell(row=r_,column=8).fill  = fl(CREAM)
    ws_dash.cell(row=r_,column=9).fill  = fl(CREAM)

kpi_defs = [
    # (label, formula, fmt, label_bg, rows_span=(r1,c1,r2,c2))
    ("ANNUAL INCOME",  "=B10",       MONEY, SAGE_DARK,  3, 1, 4, 3),
    ("LEFT TO SPEND",  "=IFERROR(B11-B12,0)", MONEY, TERRA, 3, 5, 4, 7),
    ("ANNUAL SAVINGS", YTD_F,        MONEY, SAGE_MID,   5, 1, 6, 3),
    ("SAVINGS RATE",   "=IFERROR(B15/B10,0)", PCT,  TERRA_LIGHT, 5, 5, 6, 7),
]
for lbl,formula,fmt,bg,r1,c1,r2,c2 in kpi_defs:
    lbl_color = WHITE if bg != TERRA_LIGHT else TERRA
    val_color = WHITE if bg != TERRA_LIGHT else TERRA
    # Label row
    mc(ws_dash,r1,c1,r1,c2,lbl,True,9,lbl_color,bg,"center",
       border=thin_border())
    fill_merged_neighbors(ws_dash,r1,c1,c2,bg)
    # Value row
    mc(ws_dash,r1+1,c1,r1+1,c2,formula,True,16,val_color,bg,"center",
       fmt=fmt,border=gold_bottom_border())
    fill_merged_neighbors(ws_dash,r1+1,c1,c2,bg)

# ── Row 7: Spacer ─────────────────────────────────────────────────────────
rh(ws_dash,7,12)
for col in range(1,10):
    ws_dash.cell(row=7,column=col).fill = fl(CREAM)

# ── Row 8: Section headers ────────────────────────────────────────────────
mc(ws_dash,8,1,8,3,"BUDGET SUMMARY",True,10,WHITE,SAGE_MID,"left",indent=1)
fill_merged_neighbors(ws_dash,8,1,3,SAGE_MID)
mc(ws_dash,8,5,8,7,"SAVINGS SNAPSHOT",True,10,WHITE,TERRA,"left",indent=1)
fill_merged_neighbors(ws_dash,8,5,7,TERRA)
for col in [4,8,9]: ws_dash.cell(row=8,column=col).fill = fl(CREAM)
rh(ws_dash,8,ROW_SECTION)

# ── Row 9: Column headers ─────────────────────────────────────────────────
for ci,h in [(1,"METRIC"),(2,""),(3,"VALUE")]:
    sc(ws_dash,9,ci,h,True,9,WHITE,SAGE_DARK,"center")
for ci,h in [(5,"METRIC"),(6,""),(7,"VALUE")]:
    sc(ws_dash,9,ci,h,True,9,WHITE,TERRA,"center")
for col in [4,8,9]: ws_dash.cell(row=9,column=col).fill = fl(CREAM)
rh(ws_dash,9,ROW_COLHDR)

# ── Rows 10–15: Budget Summary + Savings Snapshot ─────────────────────────
bs_rows = [
    ("Annual Income",        True,  "=B10",                             MONEY, INPUT_BLUE),
    ("Monthly Income",       False, "=IFERROR(B10/12,0)",               MONEY, CHARCOAL),
    ("This Month Expenses",  False, f"=IFERROR(Jan!D{jan_gt},0)",       MONEY, GREEN),
    ("Left to Spend",        False, "=IFERROR(B11-B12,0)",              MONEY, CHARCOAL),
    ("Annual Savings Rate",  False, "=IFERROR((B10-B12*12)/B10,0)",     PCT,   CHARCOAL),
    ("YTD Savings",          False, YTD_F,                              MONEY, GREEN),
]
ss_rows = [
    ("Emergency Fund Goal",    True,  "=F10",                              MONEY, INPUT_BLUE),
    ("Emergency Fund Current", True,  "=F11",                              MONEY, INPUT_BLUE),
    ("% Funded",               False, "=IFERROR(F11/F10,0)",               PCT,   CHARCOAL),
    ("Monthly Target",         True,  "=F13",                              MONEY, INPUT_BLUE),
    ("YTD Saved",              False, YTD_F,                              MONEY, GREEN),
    ("Months to Goal",         False, "=IFERROR((F10-F11)/F13,0)",         '0.0";-"',  CHARCOAL),
]

for i in range(6):
    r = 10 + i
    bg_bs = SAGE_PALE if i%2==0 else WHITE
    bg_ss = TERRA_LIGHT if i%2==0 else WHITE

    # Budget Summary left (A:C)
    lbl_t, is_inp, formula, fmt, color = bs_rows[i]
    ws_dash.merge_cells(start_row=r,start_column=1,end_row=r,end_column=2)
    sc(ws_dash,r,1,lbl_t,True,9,CHARCOAL,bg_bs,"left")
    ws_dash.cell(row=r,column=2).fill   = fl(bg_bs)
    ws_dash.cell(row=r,column=2).border = thin_border()
    if is_inp:
        inp(ws_dash,r,3,0,fmt,bg_bs)
    else:
        frm(ws_dash,r,3,formula,fmt,color,bg_bs)
    rh(ws_dash,r,ROW_DATA)

    # Spacer
    ws_dash.cell(row=r,column=4).fill = fl(CREAM)

    # Savings Snapshot right (E:G)
    lbl_t2,is_inp2,formula2,fmt2,color2 = ss_rows[i]
    ws_dash.merge_cells(start_row=r,start_column=5,end_row=r,end_column=6)
    sc(ws_dash,r,5,lbl_t2,True,9,CHARCOAL,bg_ss,"left")
    ws_dash.cell(row=r,column=6).fill   = fl(bg_ss)
    ws_dash.cell(row=r,column=6).border = thin_border()
    if is_inp2:
        inp(ws_dash,r,7,0,fmt2,bg_ss)
    else:
        frm(ws_dash,r,7,formula2,fmt2,color2,bg_ss)
    for col in [8,9]: ws_dash.cell(row=r,column=col).fill = fl(CREAM)

# ── Row 16: Spacer ────────────────────────────────────────────────────────
rh(ws_dash,16,12)
for col in range(1,10): ws_dash.cell(row=16,column=col).fill = fl(CREAM)

# ── Row 17: Top Categories + Bills headers ────────────────────────────────
mc(ws_dash,17,1,17,3,"TOP EXPENSE CATEGORIES — JANUARY",True,10,WHITE,SAGE_MID,"left",indent=1)
fill_merged_neighbors(ws_dash,17,1,3,SAGE_MID)
mc(ws_dash,17,5,17,7,"BILLS THIS MONTH",True,10,WHITE,TERRA,"left",indent=1)
fill_merged_neighbors(ws_dash,17,5,7,TERRA)
for col in [4,8,9]: ws_dash.cell(row=17,column=col).fill = fl(CREAM)
rh(ws_dash,17,ROW_SECTION)

# ── Row 18: Sub-headers ───────────────────────────────────────────────────
for ci,h in [(1,"CATEGORY"),(2,"BUDGETED"),(3,"ACTUAL")]:
    sc(ws_dash,18,ci,h,True,9,WHITE,SAGE_DARK,"center")
for ci,h in [(5,"BILL NAME"),(6,"AMOUNT"),(7,"DUE")]:
    sc(ws_dash,18,ci,h,True,9,WHITE,TERRA,"center")
for col in [4,8,9]: ws_dash.cell(row=18,column=col).fill = fl(CREAM)
rh(ws_dash,18,ROW_COLHDR)

# ── Rows 19–24: Top 6 categories + sample bills ───────────────────────────
bills = [
    ("Hyra/Mortgage",      12500, "1st"),
    ("El",                   850, "15th"),
    ("Internet",             499, "22nd"),
    ("Streaming",            139, "8th"),
    ("Telefon",              699, "12th"),
    ("Hemförsäkring",       1200, "1st"),
]
for i,(cat_name,_) in enumerate(CATS[:6]):
    r = 19 + i
    bg  = SAGE_PALE if i%2==0 else WHITE
    bgb = TERRA_LIGHT if i%2==0 else WHITE
    sub_r = rows["Jan"][f"{cat_name}_total"]

    sc(ws_dash,r,1,cat_name,True,9,CHARCOAL,bg,"left")
    frm(ws_dash,r,2,f"=IFERROR(Jan!C{sub_r},0)",MONEY,GREEN,bg)
    frm(ws_dash,r,3,f"=IFERROR(Jan!D{sub_r},0)",MONEY,GREEN,bg)
    ws_dash.cell(row=r,column=4).fill = fl(CREAM)

    bn,ba,bd = bills[i]
    sc(ws_dash,r,5,bn,bg=bgb,h_align="left")
    inp(ws_dash,r,6,ba,MONEY,bgb)
    sc(ws_dash,r,7,bd,bg=bgb,h_align="center")
    for col in [8,9]: ws_dash.cell(row=r,column=col).fill = fl(CREAM)
    rh(ws_dash,r,ROW_DATA)

# ── Row 26: Spacer ────────────────────────────────────────────────────────
rh(ws_dash,26,12)
for col in range(1,10): ws_dash.cell(row=26,column=col).fill = fl(CREAM)

# ── Rows 27–50: Chart placeholders ───────────────────────────────────────
chart_sections = [
    (27,27,  SAGE_MID,   "CHART AREA 1 — Expense Distribution  ·  Add Donut Chart Here"),
    (28,35,  SAGE_PALE,  "INSERT DONUT CHART HERE\n\nStep 1: Go to any monthly sheet (e.g. Jan)\nStep 2: Hold Ctrl, select category names (col A) and their subtotals (col C) on each subtotal row\nStep 3: Insert > Chart > Doughnut\nStep 4: Copy chart here"),
    (36,36,  TERRA_LIGHT,""),
    (37,37,  SAGE_MID,   "CHART AREA 2 — Annual Income vs Expenses  ·  Add Bar Chart Here"),
    (38,50,  SAGE_PALE,  "INSERT CLUSTERED COLUMN CHART HERE\n\nStep 1: Go to Annual Overview sheet\nStep 2: Select INCOME row (row 4) and GRAND TOTAL row (row 14) + month columns B–M\nStep 3: Insert > Chart > Clustered Column\nStep 4: Copy chart here\n\nColors: Income = #3D5C3A  ·  Expenses = #B05E3A"),
]
for r1,r2,bg_hex,txt in chart_sections:
    if r1 == r2:
        mc(ws_dash,r1,1,r1,9,txt,True,9,WHITE,bg_hex,"left",indent=1)
        fill_merged_neighbors(ws_dash,r1,1,9,bg_hex)
        rh(ws_dash,r1,ROW_SECTION)
    else:
        mc(ws_dash,r1,1,r2,9,txt,False,9,SAGE_MID,bg_hex,"center",wrap=True)
        for rr in range(r1,r2+1):
            fill_merged_neighbors(ws_dash,rr,1,9,bg_hex)
            rh(ws_dash,rr,ROW_DATA)
        set_outer_border_range(ws_dash,r1,1,r2,9,SAGE_MID)

ws_dash.freeze_panes = "A3"
ws_dash.column_dimensions["A"].width = 24
ws_dash.column_dimensions["B"].width = 16
ws_dash.column_dimensions["C"].width = 14
ws_dash.column_dimensions["D"].width = 3
ws_dash.column_dimensions["E"].width = 24
ws_dash.column_dimensions["F"].width = 16
ws_dash.column_dimensions["G"].width = 14
ws_dash.column_dimensions["H"].width = 3
ws_dash.column_dimensions["I"].width = 16


# ═══════════════════════════════════════════════════════════════════════════
# DEBT TRACKER
# ═══════════════════════════════════════════════════════════════════════════
ws_debt = wb.create_sheet("Debt Tracker")
no_gridlines(ws_debt)
ws_debt.sheet_properties.tabColor = TERRA

mc(ws_debt,1,1,1,9,"DEBT PAYOFF TRACKER",True,16,WHITE,TERRA,"center")
rh(ws_debt,1,ROW_TITLE)
mc(ws_debt,2,1,2,9,
   "Track balances and interest  ·  Avalanche = highest rate first",
   False,9,WHITE,GOLD,"left",italic=True)
rh(ws_debt,2,ROW_SUBTITLE)
for ci,h in enumerate(["DEBT NAME","LENDER","ORIGINAL BAL","CURRENT BAL",
                        "INTEREST %","MIN PAYMENT","MONTHLY PMT","EST PAYOFF","STATUS"]):
    sc(ws_debt,3,ci+1,h,True,9,WHITE,SAGE_DARK,"center")
rh(ws_debt,3,ROW_COLHDR)

debts = ["Credit Card 1","Credit Card 2","Student Loan","Car Loan","Personal Loan","Other"]
for ji,debt in enumerate(debts):
    r = 4+ji
    bg = TERRA_LIGHT if ji%2==0 else WHITE
    sc(ws_debt,r,1,debt,bg=bg,h_align="left")
    inp(ws_debt,r,2,"","@",bg); ws_debt.cell(row=r,column=2).number_format="@"
    inp(ws_debt,r,3,0,MONEY,bg)
    inp(ws_debt,r,4,0,MONEY,bg)
    inp(ws_debt,r,5,0,PCT,bg)
    inp(ws_debt,r,6,0,MONEY,bg)
    inp(ws_debt,r,7,0,MONEY,bg)
    inp(ws_debt,r,8,"",DATE_FMT,bg)
    sc(ws_debt,r,9,"Active",bg=bg,h_align="center")
    rh(ws_debt,r,ROW_DATA)

dt_end = 4+len(debts)-1; dt_tot = dt_end+1
mc(ws_debt,dt_tot,1,dt_tot,2,"TOTALS",True,10,WHITE,TERRA,"left")
fill_merged_neighbors(ws_debt,dt_tot,1,2,TERRA)
for col,f in [(3,f"=SUM(C4:C{dt_end})"),(4,f"=SUM(D4:D{dt_end})"),
              (6,f"=SUM(F4:F{dt_end})"),(7,f"=SUM(G4:G{dt_end})")]:
    frm(ws_debt,dt_tot,col,f,MONEY,WHITE,TERRA)
for col in [5,8,9]: sc(ws_debt,dt_tot,col,"",bg=TERRA)
rh(ws_debt,dt_tot,ROW_GRAND)

# Summary box
sb = dt_tot + 2
mc(ws_debt,sb,1,sb,5,"DEBT SUMMARY",True,10,WHITE,SAGE_DARK,"left",indent=1)
fill_merged_neighbors(ws_debt,sb,1,5,SAGE_DARK)
rh(ws_debt,sb,ROW_SECTION)

debt_summary = [
    ("Total Outstanding",      f"=SUM(D4:D{dt_end})",                               MONEY),
    ("Total Monthly Minimums", f"=SUM(F4:F{dt_end})",                               MONEY),
    ("Highest Rate Debt",      f'=IFERROR(INDEX(A4:A{dt_end},MATCH(MAX(E4:E{dt_end}),E4:E{dt_end},0)),"-")', "@"),
    ("Debt-to-Income (monthly)",f"=IFERROR(SUM(G4:G{dt_end})/(Dashboard!B10/12),0)", PCT),
]
for ki,(sl,sf,sfmt) in enumerate(debt_summary):
    r = sb+1+ki
    bg = SAGE_PALE if ki%2==0 else WHITE
    ws_debt.merge_cells(start_row=r,start_column=1,end_row=r,end_column=3)
    sc(ws_debt,r,1,sl,True,9,CHARCOAL,bg,"left")
    fill_merged_neighbors(ws_debt,r,1,3,bg)
    ws_debt.merge_cells(start_row=r,start_column=4,end_row=r,end_column=5)
    frm(ws_debt,r,4,sf,sfmt,GREEN,bg)
    fill_merged_neighbors(ws_debt,r,4,5,bg)
    rh(ws_debt,r,ROW_DATA)

ws_debt.freeze_panes = "A3"
for ci,w in zip(range(1,10),[22,18,14,14,12,14,14,14,12]):
    ws_debt.column_dimensions[get_column_letter(ci)].width = w


# ═══════════════════════════════════════════════════════════════════════════
# BILL CALENDAR
# ═══════════════════════════════════════════════════════════════════════════
ws_bill = wb.create_sheet("Bill Calendar")
no_gridlines(ws_bill)
ws_bill.sheet_properties.tabColor = TERRA

mc(ws_bill,1,1,1,9,"BILL CALENDAR  ·  2026",True,16,WHITE,TERRA,"center")
rh(ws_bill,1,ROW_TITLE)
mc(ws_bill,2,1,2,9,"Never miss a payment  ·  Green = paid  ·  Red = overdue",
   False,9,WHITE,GOLD,"left",italic=True)
rh(ws_bill,2,ROW_SUBTITLE)
for ci,h in enumerate(["BILL NAME","CATEGORY","AMOUNT","DUE DATE",
                        "FREQUENCY","AUTO-PAY","PAID?","ANNUAL COST","NOTES"]):
    sc(ws_bill,3,ci+1,h,True,9,WHITE,SAGE_DARK,"center")
rh(ws_bill,3,ROW_COLHDR)

bills_data = [
    ("Hyra/Mortgage",      "Housing",    12500, "2026-01-01","Monthly","No", "No"),
    ("El",                 "Utilities",    850, "2026-01-15","Monthly","No", "No"),
    ("Internet",           "Utilities",    499, "2026-01-22","Monthly","Yes","No"),
    ("Netflix/Streaming",  "Lifestyle",    139, "2026-01-08","Monthly","Yes","No"),
    ("Telefon",            "Utilities",    699, "2026-01-12","Monthly","Yes","No"),
    ("Hemförsäkring",      "Insurance",   1200, "2026-01-01","Monthly","Yes","No"),
    ("Gym",                "Health",       400, "2026-01-05","Monthly","Yes","No"),
    ("Bilförsäkring",      "Insurance",   1100, "2026-01-20","Monthly","Yes","No"),
    ("Spotify",            "Lifestyle",    109, "2026-01-14","Monthly","Yes","No"),
    ("A-kassa/Facket",     "Other",        350, "2026-01-01","Monthly","Yes","No"),
]
for ji,(bn,bc,ba,bd,bf,bap,bpd) in enumerate(bills_data):
    r = 4+ji
    bg = TERRA_LIGHT if ji%2==0 else WHITE
    sc(ws_bill,r,1,bn,bg=bg,h_align="left")
    sc(ws_bill,r,2,bc,bg=bg,h_align="left")
    inp(ws_bill,r,3,ba,MONEY,bg)
    cd=ws_bill.cell(row=r,column=4,value=bd)
    cd.font=fn(9,False,INPUT_BLUE); cd.fill=fl(bg)
    cd.alignment=al("right"); cd.border=thin_border(); cd.number_format=DATE_FMT
    sc(ws_bill,r,5,bf,bg=bg,h_align="left")
    sc(ws_bill,r,6,bap,bg=bg,h_align="center")
    sc(ws_bill,r,7,bpd,bg=bg,h_align="center")
    ac=ws_bill.cell(row=r,column=8,
        value=f'=IFERROR(IF(E{r}="Monthly",C{r}*12,IF(E{r}="Quarterly",C{r}*4,IF(E{r}="Annual",C{r},C{r}*52))),0)')
    ac.font=fn(9,False,CHARCOAL); ac.fill=fl(bg); ac.alignment=al("right")
    ac.border=thin_border(); ac.number_format=MONEY
    sc(ws_bill,r,9,"",bg=bg)
    rh(ws_bill,r,ROW_DATA)

be=4+len(bills_data)-1; bt=be+1
mc(ws_bill,bt,1,bt,2,"MONTHLY TOTAL",True,10,WHITE,TERRA,"left")
fill_merged_neighbors(ws_bill,bt,1,2,TERRA)
frm(ws_bill,bt,3,f"=SUM(C4:C{be})",MONEY,WHITE,TERRA)
frm(ws_bill,bt,8,f"=SUM(H4:H{be})",MONEY,WHITE,TERRA)
for col in [4,5,6,7,9]: sc(ws_bill,bt,col,"",bg=TERRA)
rh(ws_bill,bt,ROW_GRAND)

# Conditional formatting
paid_rule = FormulaRule(formula=['$G3="Yes"'],
    fill=fl(SAGE_LIGHT), font=fn(9,False,SAGE_DARK))
ws_bill.conditional_formatting.add(f"A3:I{be}", paid_rule)
autopay_rule = FormulaRule(formula=['$F3="Yes"'],
    font=fn(9,True,GOLD))
ws_bill.conditional_formatting.add(f"F3:F{be}", autopay_rule)

for dv_range,formula1 in [
    (f"B4:B{be}",'"Housing,Utilities,Insurance,Lifestyle,Health,Transport,Other"'),
    (f"E4:E{be}",'"Monthly,Quarterly,Annual,Weekly"'),
    (f"F4:G{be}",'"Yes,No"'),
]:
    dv=DataValidation(type="list",formula1=formula1,allow_blank=True)
    ws_bill.add_data_validation(dv); dv.sqref=dv_range

ws_bill.freeze_panes = "A3"
for ci,w in zip(range(1,10),[22,16,12,14,12,10,8,14,22]):
    ws_bill.column_dimensions[get_column_letter(ci)].width = w


# ═══════════════════════════════════════════════════════════════════════════
# SAVINGS GOALS
# ═══════════════════════════════════════════════════════════════════════════
ws_sav = wb.create_sheet("Savings Goals")
no_gridlines(ws_sav)
ws_sav.sheet_properties.tabColor = SAGE_DARK

mc(ws_sav,1,1,1,8,"SAVINGS GOALS TRACKER",True,16,WHITE,SAGE_DARK,"center")
rh(ws_sav,1,ROW_TITLE)
mc(ws_sav,2,1,2,8,"Set targets  ·  Track progress  ·  Automate contributions",
   False,9,WHITE,GOLD,"left",italic=True)
rh(ws_sav,2,ROW_SUBTITLE)
for ci,h in enumerate(["GOAL","TARGET","SAVED SO FAR","MONTHLY CONTRIB",
                        "TARGET DATE","% COMPLETE","PROGRESS","PRIORITY"]):
    sc(ws_sav,3,ci+1,h,True,9,WHITE,SAGE_DARK,"center")
rh(ws_sav,3,ROW_COLHDR)

goals = [
    ("Emergency Fund (3mo)",50000, 0,500, "2026-12-31","High"),
    ("Japan Trip",          25000, 0,200, "2026-09-01","Medium"),
    ("Home Down Payment",  250000, 0,800, "2029-01-01","High"),
    ("New Car",             80000, 0,300, "2027-06-01","Medium"),
    ("Retirement Boost",   100000, 0,400, "2030-01-01","High"),
    ("Education Fund",      40000, 0,250, "2028-01-01","Low"),
]
pri_bg = {"High":TERRA_LIGHT,"Medium":SAGE_PALE,"Low":CREAM}
for ji,(gn,gt_,gc,gm,gd,gp) in enumerate(goals):
    r = 4+ji
    bg = pri_bg.get(gp,CREAM)
    sc(ws_sav,r,1,gn,True,9,CHARCOAL,bg,"left")
    inp(ws_sav,r,2,gt_,MONEY,bg)
    inp(ws_sav,r,3,gc, MONEY,bg)
    inp(ws_sav,r,4,gm, MONEY,bg)
    cd=ws_sav.cell(row=r,column=5,value=gd)
    cd.font=fn(9,False,INPUT_BLUE); cd.fill=fl(bg)
    cd.alignment=al("right"); cd.border=thin_border(); cd.number_format=DATE_FMT
    frm(ws_sav,r,6,f"=IFERROR(C{r}/B{r},0)",PCT,CHARCOAL,bg)
    pb=ws_sav.cell(row=r,column=7,
        value=f'=REPT(CHAR(9608),ROUND(F{r}*10,0))&REPT(CHAR(9617),10-ROUND(F{r}*10,0))')
    pb.font=Font(name="Calibri",size=11,color=SAGE_DARK)
    pb.fill=fl(bg); pb.alignment=al("left"); pb.border=thin_border()
    sc(ws_sav,r,8,gp,bold=True,bg=bg,h_align="center")
    rh(ws_sav,r,ROW_DATA)

ge=4+len(goals)-1; gt_row=ge+1
for ci,val,fmt_ in [
    (1,"TOTALS","@"),(2,f"=SUM(B4:B{ge})",MONEY),(3,f"=SUM(C4:C{ge})",MONEY),
    (4,f"=SUM(D4:D{ge})",MONEY),(6,f"=IFERROR(SUM(C4:C{ge})/SUM(B4:B{ge})",PCT+")"),
]:
    c=ws_sav.cell(row=gt_row,column=ci)
    # fix fmt for col 6
    if ci==6:
        c.value=f"=IFERROR(SUM(C4:C{ge})/SUM(B4:B{ge}),0)"
        c.number_format=PCT
    else:
        c.value=val
        c.number_format=fmt_ if fmt_ != "@" else "@"
    c.font=fn(9,True,CHARCOAL); c.fill=fl(SAGE_LIGHT)
    c.alignment=al("right" if ci>1 else "left"); c.border=section_bottom_border()
for ci in [5,7,8]:
    c=ws_sav.cell(row=gt_row,column=ci)
    c.fill=fl(SAGE_LIGHT); c.border=thin_border()
rh(ws_sav,gt_row,ROW_SUBTOTAL)

# Priority conditional formatting
for rule_val,rule_bg in [("High",TERRA_LIGHT),("Medium",SAGE_PALE),("Low",CREAM)]:
    ws_sav.conditional_formatting.add(f"H4:H{ge}",
        FormulaRule(formula=[f'$H4="{rule_val}"'],fill=fl(rule_bg)))

dv_p=DataValidation(type="list",formula1='"High,Medium,Low"',allow_blank=True)
ws_sav.add_data_validation(dv_p); dv_p.sqref=f"H4:H{ge}"

ws_sav.freeze_panes = "A3"
for ci,w in zip(range(1,9),[22,14,14,16,14,11,18,10]):
    ws_sav.column_dimensions[get_column_letter(ci)].width = w


# ═══════════════════════════════════════════════════════════════════════════
# SPENDING TRACKER
# ═══════════════════════════════════════════════════════════════════════════
ws_sp = wb.create_sheet("Spending Tracker")
no_gridlines(ws_sp)
ws_sp.sheet_properties.tabColor = SAGE_DARK

mc(ws_sp,1,1,1,33,"DAYS WITHOUT SPENDING  ·  2026",True,16,WHITE,SAGE_DARK,"center")
rh(ws_sp,1,ROW_TITLE)
mc(ws_sp,2,1,2,33,"Mark each no-spend day with X  ·  Watch your streak grow",
   False,9,WHITE,GOLD,"left",italic=True)
rh(ws_sp,2,ROW_SUBTITLE)

# Row 3: headers
sc(ws_sp,3,1,"MONTH",True,9,WHITE,SAGE_DARK,"center")
for d in range(1,32):
    sc(ws_sp,3,d+1,d,True,9,WHITE,SAGE_MID,"center")
sc(ws_sp,3,33,"NO-SPEND DAYS",True,9,WHITE,SAGE_DARK,"center")
rh(ws_sp,3,ROW_COLHDR)

days_in_month_2026 = [31,28,31,30,31,30,31,31,30,31,30,31]

for mi,m_long in enumerate(MO_LONG):
    r = 4+mi
    bg_row = SAGE_PALE if mi%2==0 else WHITE
    sc(ws_sp,r,1,m_long,True,9,CHARCOAL,SAGE_LIGHT if mi%2==0 else TERRA_LIGHT,"left")
    days = days_in_month_2026[mi]
    for d in range(1,32):
        col = d+1
        c = ws_sp.cell(row=r,column=col)
        if d <= days:
            c.value = ""
            c.font  = fn(9,False,INPUT_BLUE)
            c.fill  = fl(bg_row)
            c.alignment = al("center")
            c.border = thin_border()
        else:
            c.fill   = fl(GRAY_DIS)
            c.border = thin_border()
    # Count formula
    cf = ws_sp.cell(row=r,column=33,
        value=f'=COUNTIF(B{r}:{get_column_letter(32)}{r},"X")')
    cf.font      = fn(9,True,GREEN)
    cf.fill      = fl(SAGE_LIGHT)
    cf.alignment = al("center")
    cf.border    = thin_border()
    rh(ws_sp,r,ROW_DATA)

# Annual total
ann_r = 4+12
mc(ws_sp,ann_r,1,ann_r,32,"ANNUAL TOTAL NO-SPEND DAYS",True,10,WHITE,SAGE_DARK,"left",indent=1)
for col in range(1,33):
    try:
        ws_sp.cell(row=ann_r,column=col).fill = fl(SAGE_DARK)
    except AttributeError:
        pass
count_sum = "+".join([f"AG{4+mi}" for mi in range(12)])
cf_ann = ws_sp.cell(row=ann_r,column=33,value=f"={count_sum}")
cf_ann.font=fn(10,True,WHITE); cf_ann.fill=fl(SAGE_DARK)
cf_ann.alignment=al("center"); cf_ann.border=thin_border()
rh(ws_sp,ann_r,ROW_GRAND)

# Conditional formatting: X = green
x_rule = FormulaRule(
    formula=['B4="X"'],
    fill=fl(SAGE_LIGHT),
    font=fn(9,True,SAGE_DARK)
)
ws_sp.conditional_formatting.add(f"B4:{get_column_letter(32)}{4+11}", x_rule)

ws_sp.freeze_panes = "B3"
ws_sp.column_dimensions["A"].width = 14
for col_idx in range(2,33):
    ws_sp.column_dimensions[get_column_letter(col_idx)].width = 3.5
ws_sp.column_dimensions[get_column_letter(33)].width = 16


# ═══════════════════════════════════════════════════════════════════════════
# ANNUAL OVERVIEW
# ═══════════════════════════════════════════════════════════════════════════
ws_ann = wb.create_sheet("Annual Overview")
no_gridlines(ws_ann)
ws_ann.sheet_properties.tabColor = SAGE_DARK

mc(ws_ann,1,1,1,16,"ANNUAL OVERVIEW  ·  2026",True,16,WHITE,SAGE_DARK,"center")
rh(ws_ann,1,ROW_TITLE)
mc(ws_ann,2,1,2,16,
   "Full year summary  ·  All figures pulled automatically from monthly sheets",
   False,9,WHITE,GOLD,"left",italic=True)
rh(ws_ann,2,ROW_SUBTITLE)

# Row 3: column headers
for ci,h in enumerate(["LINE ITEM"]+MO_SHORT+["ANNUAL TOTAL","ANNUAL BUDGET","VARIANCE"]):
    bg_h = SAGE_DARK if ci in [0,13,14,15] else SAGE_MID
    sc(ws_ann,3,ci+1,h,True,9,WHITE,bg_h,"center")
rh(ws_ann,3,ROW_COLHDR)

annual_items = [
    ("INCOME",                 "income_total",                    SAGE_LIGHT,  True),
    ("— Housing",              "HOUSING_total",                   SAGE_PALE,   False),
    ("— Transport",            "TRANSPORT_total",                 WHITE,       False),
    ("— Food & Dining",        "FOOD & DINING_total",             SAGE_PALE,   False),
    ("— Health & Wellness",    "HEALTH & WELLNESS_total",         WHITE,       False),
    ("— Lifestyle",            "LIFESTYLE_total",                 SAGE_PALE,   False),
    ("— Personal",             "PERSONAL_total",                  WHITE,       False),
    ("— Savings & Investments","SAVINGS & INVESTMENTS_total",     SAGE_PALE,   False),
    ("— Debt Payments",        "DEBT PAYMENTS_total",             WHITE,       False),
    ("GRAND TOTAL EXPENSES",   "grand_total",                     TERRA_LIGHT, True),
    ("NET CASH FLOW",          "net_flow",                        SAGE_DARK,   True),
]

income_ann_row   = None
grand_ann_row    = None
ncf_ann_row      = None

for i,(label,row_key,bg,is_key) in enumerate(annual_items):
    ar = 4+i
    txt = WHITE if bg == SAGE_DARK else (CHARCOAL)
    size_ = 10 if is_key else 9

    sc(ws_ann,ar,1,label,is_key,size_,txt,bg,"left",indent=(0 if is_key else 1))

    for mi,mo in enumerate(MO_SHORT):
        ref_row = rows[mo][row_key]
        fc=ws_ann.cell(row=ar,column=2+mi,value=f"={mo}!D{ref_row}")
        fc.font=fn(size_,is_key,GREEN if not is_key else (WHITE if bg==SAGE_DARK else txt))
        fc.fill=fl(bg); fc.alignment=al("right"); fc.border=thin_border()
        fc.number_format=MONEY

    # Annual Total
    fc_n=ws_ann.cell(row=ar,column=14,value=f"=SUM(B{ar}:M{ar})")
    fc_n.font=fn(size_,True,txt); fc_n.fill=fl(bg); fc_n.alignment=al("right")
    fc_n.border=thin_border(); fc_n.number_format=MONEY

    # Annual Budget (input)
    inp(ws_ann,ar,15,0,MONEY,bg)

    # Variance
    fc_v=ws_ann.cell(row=ar,column=16,value=f"=IFERROR(N{ar}-O{ar},0)")
    fc_v.font=fn(size_,is_key,txt); fc_v.fill=fl(bg); fc_v.alignment=al("right")
    fc_v.border=thin_border(); fc_v.number_format=MONEY

    if "INCOME" == label:         income_ann_row = ar
    if "GRAND TOTAL" in label:    grand_ann_row  = ar
    if "NET CASH FLOW" == label:  ncf_ann_row    = ar

    rh(ws_ann,ar,ROW_GRAND if is_key else ROW_DATA)

# KEY METRICS
km = 4+len(annual_items)+2
mc(ws_ann,km,1,km,5,"KEY METRICS",True,10,WHITE,SAGE_DARK,"left",indent=1)
fill_merged_neighbors(ws_ann,km,1,5,SAGE_DARK)
rh(ws_ann,km,ROW_SECTION)

km_items = [
    ("Best Month (lowest spend)",
     f'=IFERROR(INDEX(B3:M3,MATCH(MIN(B{grand_ann_row}:M{grand_ann_row}),B{grand_ann_row}:M{grand_ann_row},0)),"-")','@'),
    ("Worst Month (highest spend)",
     f'=IFERROR(INDEX(B3:M3,MATCH(MAX(B{grand_ann_row}:M{grand_ann_row}),B{grand_ann_row}:M{grand_ann_row},0)),"-")','@'),
    ("Annual Savings Rate",
     f"=IFERROR((N{income_ann_row}-N{grand_ann_row})/N{income_ann_row},0)", PCT),
    ("Avg Monthly Spend",
     f"=IFERROR(N{grand_ann_row}/12,0)", MONEY),
    ("Avg Monthly Income",
     f"=IFERROR(N{income_ann_row}/12,0)", MONEY),
]
for ki,(sl,sf,sfmt) in enumerate(km_items):
    r = km+1+ki
    bg = SAGE_PALE if ki%2==0 else WHITE
    ws_ann.merge_cells(start_row=r,start_column=1,end_row=r,end_column=3)
    sc(ws_ann,r,1,sl,True,9,CHARCOAL,bg,"left")
    fill_merged_neighbors(ws_ann,r,1,3,bg)
    ws_ann.merge_cells(start_row=r,start_column=4,end_row=r,end_column=5)
    frm(ws_ann,r,4,sf,sfmt,GREEN,bg)
    fill_merged_neighbors(ws_ann,r,4,5,bg)
    rh(ws_ann,r,ROW_DATA)

ws_ann.freeze_panes = "B3"
ws_ann.column_dimensions["A"].width = 26
for ci in range(2,14):
    ws_ann.column_dimensions[get_column_letter(ci)].width = 10
ws_ann.column_dimensions["N"].width = 13
ws_ann.column_dimensions["O"].width = 12
ws_ann.column_dimensions["P"].width = 12


# ═══════════════════════════════════════════════════════════════════════════
# SHEET ORDER CHECK
# ═══════════════════════════════════════════════════════════════════════════
expected = (["Dashboard"] + MO_SHORT +
            ["Debt Tracker","Bill Calendar","Savings Goals","Spending Tracker","Annual Overview"])
assert wb.sheetnames == expected, f"Sheet order mismatch:\n  Got:      {wb.sheetnames}\n  Expected: {expected}"


# ═══════════════════════════════════════════════════════════════════════════
# VERIFICATION PRINT
# ═══════════════════════════════════════════════════════════════════════════
print("\n✅ VERIFIERING")
print(f"   Antal ark: {len(wb.sheetnames)}")
for name in wb.sheetnames:
    ws_ = wb[name]
    print(f"   {name}: {ws_.max_row} rader, showGridLines={ws_.sheet_view.showGridLines}")

print("\n📍 RADNYCKEL (Jan):")
for key,val in sorted(rows["Jan"].items()):
    print(f"   Jan.{key} = rad {val}")

print("\n💾 Sparar ultimate_budget_v4.xlsx...")
wb.save("ultimate_budget_v4.xlsx")
print("   ✅ Klar!\n")

print("=" * 60)
print("=== LÄGG TILL DIAGRAM I EXCEL ===")
print()
print("1. DONUT-DIAGRAM (Expense Distribution):")
print("   - Gå till Jan-arket")
jan_cat_rows = [(cat, rows['Jan'][f'{cat}_total']) for cat,_ in CATS]
print("   - Subtotalrader (Actual = kolumn D):")
for cat,row_n in jan_cat_rows:
    print(f"       {cat}: rad {row_n}")
print("   - Håll Ctrl och klicka cell A och D på varje subtotalrad")
print("   - Insert > Chart > Doughnut")
print("   - Kopiera diagrammet till Dashboard, platshållare 'CHART AREA 1'")
print()
print("2. STAPELDIAGRAM (Annual Income vs Expenses):")
print(f"   - Gå till Annual Overview")
print(f"   - INCOME rad: {income_ann_row}  |  GRAND TOTAL rad: {grand_ann_row}")
print(f"   - Markera B{income_ann_row}:M{income_ann_row} + B{grand_ann_row}:M{grand_ann_row} (Ctrl+klick)")
print("   - Insert > Chart > Clustered Column")
print("   - Färger: Income = #3D5C3A  ·  Expenses = #B05E3A")
print("   - Flytta till Dashboard, platshållare 'CHART AREA 2'")
print()
print("3. SPARKLINES (valfritt):")
print("   - Annual Overview, markera N-kolumnen (Annual Total)")
print("   - Insert > Sparklines > Line")
print()
print("Total tid: ~10 minuter")
print("=" * 60)
