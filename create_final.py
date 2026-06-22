"""
Ultimate Budget Final — ultimate_budget_final.xlsx
openpyxl-only (xlwings skipped: requires desktop Excel on Windows/Mac).
Full structure from v4 + embedded openpyxl charts:
  - Dashboard: Donut (expense distribution) + Clustered Column (annual)
  - Each monthly sheet: horizontal Bar (budget vs actual)
"""

import calendar
from openpyxl import Workbook
from openpyxl.styles import Font, PatternFill, Alignment, Border, Side
from openpyxl.utils import get_column_letter
from openpyxl.worksheet.datavalidation import DataValidation
from openpyxl.formatting.rule import FormulaRule
from openpyxl.chart import BarChart, DoughnutChart, LineChart, Reference
from openpyxl.chart.series import SeriesLabel
from openpyxl.chart.marker import Marker

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
GRAY_DIS    = "EEEEEE"
BORDER_GRAY = "DCDCDC"

MONEY = '#,##0;[RED]-#,##0;"-"'
PCT   = '0.0%;[RED]-0.0%;"-"'

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

# Category chart colours (series fill)
CAT_COLORS = [SAGE_DARK, SAGE_MID, TERRA, GOLD, SAGE_LIGHT,
              TERRA_LIGHT, "7A9E76", "D4A373"]

# ─── STYLE HELPERS ──────────────────────────────────────────────────────────
def fl(h):  return PatternFill("solid", fgColor=h)
def fn(size=9, bold=False, color=CHARCOAL, italic=False):
    return Font(name="Calibri", size=size, bold=bold, color=color, italic=italic)
def al(h="left", v="center", wrap=False, indent=0):
    return Alignment(horizontal=h, vertical=v, wrap_text=wrap, indent=indent)

THIN  = Side(style="thin",   color=BORDER_GRAY)
GOLD_THICK = Side(style="medium", color=GOLD)
SAGE_THICK = Side(style="medium", color=SAGE_MID)

def tborder():    return Border(left=THIN,right=THIN,top=THIN,bottom=THIN)
def gold_bot():   return Border(left=THIN,right=THIN,top=THIN,bottom=GOLD_THICK)
def sage_bot():   return Border(left=THIN,right=THIN,top=THIN,bottom=SAGE_THICK)
def med_box(c=SAGE_MID):
    s=Side(style="medium",color=c)
    return Border(left=s,right=s,top=s,bottom=s)

def no_grid(ws): ws.sheet_view.showGridLines = False
def rh(ws,r,h):  ws.row_dimensions[r].height = h

def sc(ws,row,col,value="",bold=False,size=9,color=CHARCOAL,
       bg=WHITE,h="left",fmt=None,italic=False,border=None,wrap=False,indent=0):
    c = ws.cell(row=row,column=col,value=value)
    c.font      = fn(size,bold,color,italic)
    c.fill      = fl(bg)
    c.alignment = al(h,"center",wrap,indent)
    c.border    = border if border else tborder()
    if fmt: c.number_format=fmt
    return c

def mc(ws,r1,c1,r2,c2,value="",bold=False,size=9,color=CHARCOAL,
       bg=WHITE,h="center",fmt=None,italic=False,border=None,wrap=False,indent=0):
    ws.merge_cells(start_row=r1,start_column=c1,end_row=r2,end_column=c2)
    c = ws.cell(row=r1,column=c1,value=value)
    c.font      = fn(size,bold,color,italic)
    c.fill      = fl(bg)
    c.alignment = al(h,"center",wrap,indent)
    c.border    = border if border else tborder()
    if fmt: c.number_format=fmt
    return c

def inp(ws,row,col,value=0,fmt=MONEY,bg=WHITE):
    return sc(ws,row,col,value,False,9,INPUT_BLUE,bg,"right",fmt)

def frm(ws,row,col,formula,fmt=MONEY,color=CHARCOAL,bg=WHITE):
    return sc(ws,row,col,formula,False,9,color,bg,"right",fmt)

def fill_span(ws,row,c1,c2,bg):
    for col in range(c1,c2+1):
        try:
            ws.cell(row=row,column=col).fill   = fl(bg)
            ws.cell(row=row,column=col).border = tborder()
        except AttributeError:
            pass

def set_outer(ws,r1,c1,r2,c2,color=SAGE_MID):
    s=Side(style="medium",color=color)
    for row in range(r1,r2+1):
        for col in range(c1,c2+1):
            l = s if col==c1 else THIN
            ri= s if col==c2 else THIN
            t = s if row==r1 else THIN
            b = s if row==r2 else THIN
            try: ws.cell(row,col).border=Border(left=l,right=ri,top=t,bottom=b)
            except AttributeError: pass

def apply_chart_style(chart):
    chart.style = 10
    if hasattr(chart,"legend") and chart.legend:
        chart.legend.position = "b"

# ─── CHART COLOUR HELPER ─────────────────────────────────────────────────────
def hex_to_solidFill(series, hex_color):
    try:
        series.graphicalProperties.solidFill = hex_color
    except Exception:
        pass

# ════════════════════════════════════════════════════════════════════════════
# BUILD WORKBOOK
# ════════════════════════════════════════════════════════════════════════════
wb   = Workbook()
rows = {}   # rows[mo][key] = row_number
ann_data_rows = {}

ws_dash = wb.active
ws_dash.title = "Dashboard"

# ════════════════════════════════════════════════════════════════════════════
# MONTHLY SHEETS
# ════════════════════════════════════════════════════════════════════════════
for mo, m_long in zip(MO_SHORT, MO_LONG):
    ws = wb.create_sheet(title=mo)
    no_grid(ws); ws.freeze_panes="A4"
    ws.sheet_properties.tabColor = SAGE_MID
    rows[mo] = {}

    ws.column_dimensions["A"].width=24; ws.column_dimensions["B"].width=22
    for ci,w in [(3,14),(4,14),(5,14),(6,11),(7,20)]:
        ws.column_dimensions[get_column_letter(ci)].width=w

    # Title + subtitle
    mc(ws,1,1,1,7,f"{m_long.upper()}  ·  MONTHLY BUDGET",True,16,WHITE,SAGE_DARK,"center"); rh(ws,1,48)
    mc(ws,2,1,2,7,f"Track your income and expenses for {m_long}",False,9,WHITE,GOLD,"center",italic=True); rh(ws,2,20)
    for ci,h in enumerate(["CATEGORY","SUBCATEGORY","BUDGETED","ACTUAL","DIFFERENCE","% USED","NOTES"],1):
        sc(ws,3,ci,h,True,9,WHITE,SAGE_DARK,"center"); rh(ws,3,22)

    r=4

    # Income
    mc(ws,r,1,r,7,"INCOME",True,10,WHITE,TERRA,"left",indent=1); rh(ws,r,22)
    rows[mo]["income_section"]=r; r+=1
    rows[mo]["income_start"]=r
    for ji,sub in enumerate(["Primary Income","Side Income","Other Income"]):
        bg=SAGE_PALE if ji%2==0 else WHITE
        sc(ws,r,1,"INCOME",bg=bg,h="left"); sc(ws,r,2,sub,bg=bg)
        inp(ws,r,3,bg=bg); inp(ws,r,4,bg=bg)
        frm(ws,r,5,f"=C{r}-D{r}",MONEY,CHARCOAL,bg)
        frm(ws,r,6,f"=IFERROR(D{r}/C{r},0)",PCT,CHARCOAL,bg)
        sc(ws,r,7,"",bg=bg); rh(ws,r,18); r+=1
    rows[mo]["income_end"]=r-1

    ti=r
    mc(ws,ti,1,ti,2,"TOTAL INCOME",True,9,CHARCOAL,SAGE_LIGHT,"left",border=sage_bot()); fill_span(ws,ti,1,2,SAGE_LIGHT)
    frm(ws,ti,3,f"=SUM(C{rows[mo]['income_start']}:C{rows[mo]['income_end']})",MONEY,CHARCOAL,SAGE_LIGHT)
    frm(ws,ti,4,f"=SUM(D{rows[mo]['income_start']}:D{rows[mo]['income_end']})",MONEY,CHARCOAL,SAGE_LIGHT)
    frm(ws,ti,5,f"=C{ti}-D{ti}",MONEY,CHARCOAL,SAGE_LIGHT)
    frm(ws,ti,6,f"=IFERROR(D{ti}/C{ti},0)",PCT,CHARCOAL,SAGE_LIGHT)
    sc(ws,ti,7,"",bg=SAGE_LIGHT); rh(ws,ti,20)
    rows[mo]["income_total"]=ti; r+=1

    sub_c=[]; sub_d=[]
    for cat,subs in CATS:
        mc(ws,r,1,r,7,cat,True,10,WHITE,SAGE_MID,"left",indent=1); rh(ws,r,22)
        rows[mo][f"{cat}_section"]=r; r+=1
        cs=r
        for ji,sub in enumerate(subs):
            bg=SAGE_PALE if ji%2==0 else WHITE
            sc(ws,r,1,cat,bg=bg,h="left"); sc(ws,r,2,sub,bg=bg)
            inp(ws,r,3,bg=bg); inp(ws,r,4,bg=bg)
            frm(ws,r,5,f"=C{r}-D{r}",MONEY,CHARCOAL,bg)
            frm(ws,r,6,f"=IFERROR(D{r}/C{r},0)",PCT,CHARCOAL,bg)
            sc(ws,r,7,"",bg=bg); rh(ws,r,18); r+=1
        ce=r-1
        rows[mo][f"{cat}_start"]=cs; rows[mo][f"{cat}_end"]=ce
        mc(ws,r,1,r,2,f"Subtotal — {cat}",True,9,CHARCOAL,SAGE_LIGHT,"left",border=sage_bot()); fill_span(ws,r,1,2,SAGE_LIGHT)
        frm(ws,r,3,f"=SUM(C{cs}:C{ce})",MONEY,CHARCOAL,SAGE_LIGHT)
        frm(ws,r,4,f"=SUM(D{cs}:D{ce})",MONEY,CHARCOAL,SAGE_LIGHT)
        frm(ws,r,5,f"=C{r}-D{r}",MONEY,CHARCOAL,SAGE_LIGHT)
        frm(ws,r,6,f"=IFERROR(D{r}/C{r},0)",PCT,CHARCOAL,SAGE_LIGHT)
        sc(ws,r,7,"",bg=SAGE_LIGHT); rh(ws,r,20)
        rows[mo][f"{cat}_total"]=r; sub_c.append(f"C{r}"); sub_d.append(f"D{r}"); r+=1

    gt=r
    mc(ws,gt,1,gt,2,"GRAND TOTAL EXPENSES",True,10,WHITE,TERRA,"left"); fill_span(ws,gt,1,2,TERRA)
    frm(ws,gt,3,"="+"+".join(sub_c),MONEY,WHITE,TERRA)
    frm(ws,gt,4,"="+"+".join(sub_d),MONEY,WHITE,TERRA)
    frm(ws,gt,5,f"=C{gt}-D{gt}",MONEY,WHITE,TERRA)
    sc(ws,gt,6,"",bg=TERRA); sc(ws,gt,7,"",bg=TERRA); rh(ws,gt,24)
    rows[mo]["grand_total"]=gt; r+=1

    ncf=r
    mc(ws,ncf,1,ncf,2,"NET CASH FLOW",True,11,WHITE,SAGE_DARK,"left"); fill_span(ws,ncf,1,2,SAGE_DARK)
    frm(ws,ncf,3,f"=C{ti}-C{gt}",MONEY,WHITE,SAGE_DARK)
    frm(ws,ncf,4,f"=D{ti}-D{gt}",MONEY,WHITE,SAGE_DARK)
    frm(ws,ncf,5,f"=C{ncf}-D{ncf}",MONEY,WHITE,SAGE_DARK)
    sc(ws,ncf,6,"",bg=SAGE_DARK); sc(ws,ncf,7,"",bg=SAGE_DARK); rh(ws,ncf,26)
    rows[mo]["net_flow"]=ncf

    # ── MONTHLY BUDGET vs ACTUAL CHART ────────────────────────────────────
    # Helper table: cat names + subtotals (cols I-K, after data)
    helper_r_start = ncf + 2
    for ci,lbl in zip([9,10,11],["Category","Budgeted","Actual"]):
        ws.cell(helper_r_start,ci).value=lbl
        ws.cell(helper_r_start,ci).font=fn(9,True,WHITE); ws.cell(helper_r_start,ci).fill=fl(SAGE_MID)
        ws.cell(helper_r_start,ci).alignment=al("center"); ws.cell(helper_r_start,ci).border=tborder()
    rh(ws,helper_r_start,18)
    for ji,(cat,_) in enumerate(CATS):
        hr = helper_r_start+1+ji
        sub_r = rows[mo][f"{cat}_total"]
        bg = SAGE_PALE if ji%2==0 else WHITE
        sc(ws,hr,9,cat,bg=bg,h="left"); rh(ws,hr,18)
        frm(ws,hr,10,f"=C{sub_r}",MONEY,CHARCOAL,bg)
        frm(ws,hr,11,f"=D{sub_r}",MONEY,CHARCOAL,bg)
    helper_r_end = helper_r_start+len(CATS)

    bar = BarChart()
    bar.type="bar"; bar.grouping="clustered"; bar.style=10
    bar.title=f"{m_long} · Budget vs Actual"
    bar.y_axis.title=""; bar.x_axis.title="Amount"
    bar.height=14; bar.width=20
    cats_ref = Reference(ws,min_col=9,min_row=helper_r_start+1,max_row=helper_r_end)
    budg_ref = Reference(ws,min_col=10,min_row=helper_r_start,max_row=helper_r_end)
    act_ref  = Reference(ws,min_col=11,min_row=helper_r_start,max_row=helper_r_end)
    bar.add_data(budg_ref,titles_from_data=True)
    bar.add_data(act_ref, titles_from_data=True)
    bar.set_categories(cats_ref)
    hex_to_solidFill(bar.series[0],SAGE_DARK)
    hex_to_solidFill(bar.series[1],TERRA)
    apply_chart_style(bar)
    ws.add_chart(bar,f"A{ncf+2}")


# ════════════════════════════════════════════════════════════════════════════
# ANNUAL OVERVIEW
# ════════════════════════════════════════════════════════════════════════════
ws_ann = wb.create_sheet("Annual Overview")
no_grid(ws_ann); ws_ann.freeze_panes="B3"; ws_ann.sheet_properties.tabColor=SAGE_DARK
ws_ann.column_dimensions["A"].width=28
for ci in range(2,15): ws_ann.column_dimensions[get_column_letter(ci)].width=10
for ci,w in [(14,13),(15,12),(16,12)]: ws_ann.column_dimensions[get_column_letter(ci)].width=w

mc(ws_ann,1,1,1,16,"ANNUAL OVERVIEW  ·  2026",True,16,WHITE,SAGE_DARK,"center"); rh(ws_ann,1,48)
mc(ws_ann,2,1,2,16,"All figures linked automatically from monthly sheets",False,9,WHITE,GOLD,"center",italic=True); rh(ws_ann,2,20)
for ci,h in enumerate(["LINE ITEM"]+MO_SHORT+["ANNUAL TOTAL","ANNUAL BUDGET","VARIANCE"],1):
    sc(ws_ann,3,ci,h,True,9,WHITE,TERRA if ci>=15 else SAGE_DARK,"center"); rh(ws_ann,3,22)

annual_items=[
    ("INCOME",                 "income_total",                  SAGE_LIGHT,  True),
    ("— Housing",              "HOUSING_total",                 SAGE_PALE,   False),
    ("— Transport",            "TRANSPORT_total",               WHITE,       False),
    ("— Food & Dining",        "FOOD & DINING_total",           SAGE_PALE,   False),
    ("— Health & Wellness",    "HEALTH & WELLNESS_total",       WHITE,       False),
    ("— Lifestyle",            "LIFESTYLE_total",               SAGE_PALE,   False),
    ("— Personal",             "PERSONAL_total",                WHITE,       False),
    ("— Savings & Investments","SAVINGS & INVESTMENTS_total",   SAGE_PALE,   False),
    ("— Debt Payments",        "DEBT PAYMENTS_total",           WHITE,       False),
    ("GRAND TOTAL EXPENSES",   "grand_total",                   TERRA_LIGHT, True),
    ("NET CASH FLOW",          "net_flow",                      SAGE_DARK,   True),
]
income_ann=None; grand_ann=None; ncf_ann=None
ar=4
for label,key,bg,is_key in annual_items:
    txt=WHITE if bg==SAGE_DARK else CHARCOAL
    sc(ws_ann,ar,1,label,is_key,10 if is_key else 9,txt,bg,"left",
       indent=0 if is_key else 1); rh(ws_ann,ar,22 if is_key else 18)
    for mi,mo in enumerate(MO_SHORT):
        ref_r=rows[mo][key]
        fc=ws_ann.cell(row=ar,column=2+mi,value=f"={mo}!D{ref_r}")
        fc.font=fn(9,is_key,GREEN if not is_key else (WHITE if bg==SAGE_DARK else txt))
        fc.fill=fl(bg); fc.alignment=al("right"); fc.border=tborder(); fc.number_format=MONEY
    fc_n=ws_ann.cell(row=ar,column=14,value=f"=SUM(B{ar}:M{ar})")
    fc_n.font=fn(9,True,txt); fc_n.fill=fl(bg); fc_n.alignment=al("right"); fc_n.border=tborder(); fc_n.number_format=MONEY
    inp(ws_ann,ar,15,0,MONEY,bg)
    fc_v=ws_ann.cell(row=ar,column=16,value=f"=IFERROR(N{ar}-O{ar},0)")
    fc_v.font=fn(9,is_key,txt); fc_v.fill=fl(bg); fc_v.alignment=al("right"); fc_v.border=tborder(); fc_v.number_format=MONEY
    ann_data_rows[label]=ar
    if "INCOME"==label:             income_ann=ar
    if "GRAND TOTAL" in label:      grand_ann=ar
    if "NET CASH FLOW"==label:      ncf_ann=ar
    ar+=1

# KEY METRICS
km=ar+1
mc(ws_ann,km,1,km,5,"KEY METRICS",True,10,WHITE,SAGE_DARK,"left",indent=1); fill_span(ws_ann,km,1,5,SAGE_DARK); rh(ws_ann,km,22)
for ki,(sl,sf,sfmt) in enumerate([
    ("Best Month (lowest spend)",f'=IFERROR(INDEX(B3:M3,MATCH(MIN(B{grand_ann}:M{grand_ann}),B{grand_ann}:M{grand_ann},0)),"-")',"@"),
    ("Worst Month (highest spend)",f'=IFERROR(INDEX(B3:M3,MATCH(MAX(B{grand_ann}:M{grand_ann}),B{grand_ann}:M{grand_ann},0)),"-")',"@"),
    ("Annual Savings Rate",f"=IFERROR((N{income_ann}-N{grand_ann})/N{income_ann},0)",PCT),
    ("Avg Monthly Spend",f"=IFERROR(N{grand_ann}/12,0)",MONEY),
    ("Avg Monthly Income",f"=IFERROR(N{income_ann}/12,0)",MONEY),
]):
    r=km+1+ki; bg=SAGE_PALE if ki%2==0 else WHITE
    ws_ann.merge_cells(start_row=r,start_column=1,end_row=r,end_column=3)
    sc(ws_ann,r,1,sl,True,9,CHARCOAL,bg,"left"); fill_span(ws_ann,r,1,3,bg)
    ws_ann.merge_cells(start_row=r,start_column=4,end_row=r,end_column=5)
    frm(ws_ann,r,4,sf,sfmt,GREEN,bg); fill_span(ws_ann,r,4,5,bg); rh(ws_ann,r,18)

# ── ANNUAL BAR CHART (Annual Overview sheet) ──────────────────────────────
ann_bc = BarChart()
ann_bc.type="col"; ann_bc.grouping="clustered"; ann_bc.style=10
ann_bc.title="Annual Budget vs Actual — All Categories"
ann_bc.height=14; ann_bc.width=26
bc_cats = Reference(ws_ann,min_col=1,min_row=income_ann,max_row=grand_ann)
bc_act  = Reference(ws_ann,min_col=14,min_row=income_ann-1,max_row=grand_ann)
bc_budg = Reference(ws_ann,min_col=15,min_row=income_ann-1,max_row=grand_ann)
ann_bc.add_data(bc_act, titles_from_data=True)
ann_bc.add_data(bc_budg,titles_from_data=True)
ann_bc.set_categories(bc_cats)
hex_to_solidFill(ann_bc.series[0],SAGE_DARK); hex_to_solidFill(ann_bc.series[1],TERRA)
apply_chart_style(ann_bc)
ws_ann.add_chart(ann_bc,f"A{km+8}")

# ── NCF LINE CHART (Annual Overview sheet) ────────────────────────────────
ncf_lc = LineChart()
ncf_lc.style=10; ncf_lc.title="Net Cash Flow — Monthly Trend"
ncf_lc.height=10; ncf_lc.width=22
ncf_ref  = Reference(ws_ann,min_col=2,min_row=ncf_ann,max_col=13)
ncf_cats = Reference(ws_ann,min_col=2,min_row=3,max_col=13)
ncf_lc.add_data(ncf_ref); ncf_lc.set_categories(ncf_cats)
hex_to_solidFill(ncf_lc.series[0],SAGE_DARK)
apply_chart_style(ncf_lc)
ws_ann.add_chart(ncf_lc,f"J{km+8}")


# ════════════════════════════════════════════════════════════════════════════
# DASHBOARD
# ════════════════════════════════════════════════════════════════════════════
no_grid(ws_dash); ws_dash.freeze_panes="A3"; ws_dash.sheet_properties.tabColor=SAGE_DARK
for col,w in [("A",24),("B",18),("C",14),("D",3),("E",24),("F",18),("G",14),("H",3),("I",16)]:
    ws_dash.column_dimensions[col].width=w

mc(ws_dash,1,1,1,9,"PERSONAL FINANCE COMMAND CENTER  ·  2026",True,18,WHITE,SAGE_DARK,"center"); rh(ws_dash,1,50)
mc(ws_dash,2,1,2,9,"Live Summary  ·  All figures update automatically from monthly sheets",False,9,WHITE,GOLD,"center",italic=True); rh(ws_dash,2,20)

# YTD savings formula
sav_cat="SAVINGS & INVESTMENTS"
ytd_f="=IFERROR("+"+".join([f"{m}!D{rows[m][f'{sav_cat}_total']}" for m in MO_SHORT])+",0)"

jan_gt=rows["Jan"]["grand_total"]; jan_ti=rows["Jan"]["income_total"]

# KPI cards (rows 3-6)
kpi_defs=[
    ("ANNUAL INCOME","=B10",MONEY,SAGE_DARK,3,1,4,3),
    ("LEFT TO SPEND","=IFERROR(B11-B12,0)",MONEY,TERRA,3,5,4,7),
    ("ANNUAL SAVINGS",ytd_f,MONEY,SAGE_MID,5,1,6,3),
    ("SAVINGS RATE","=IFERROR((B10-B12*12)/B10,0)",PCT,GOLD,5,5,6,7),
]
for lbl,formula,fmt,bg,r1,c1,r2,c2 in kpi_defs:
    txt=WHITE if bg!=GOLD else CHARCOAL
    mc(ws_dash,r1,c1,r1,c2,lbl,True,9,txt,bg,"center"); fill_span(ws_dash,r1,c1,c2,bg); rh(ws_dash,r1,20)
    mc(ws_dash,r1+1,c1,r1+1,c2,formula,True,16,txt,bg,"center",fmt=fmt,border=gold_bot()); fill_span(ws_dash,r1+1,c1,c2,bg); rh(ws_dash,r1+1,30)
for col in [4,8,9]:
    for r_ in range(3,7):
        ws_dash.cell(r_,col).fill=fl(CREAM)

rh(ws_dash,7,10)
for col in range(1,10): ws_dash.cell(7,col).fill=fl(CREAM)

# Budget Summary + Savings Snapshot headers (row 8)
mc(ws_dash,8,1,8,3,"BUDGET SUMMARY",True,10,WHITE,SAGE_MID,"center"); fill_span(ws_dash,8,1,3,SAGE_MID)
mc(ws_dash,8,5,8,7,"SAVINGS SNAPSHOT",True,10,WHITE,TERRA,"center"); fill_span(ws_dash,8,5,7,TERRA)
for col in [4,8,9]: ws_dash.cell(8,col).fill=fl(CREAM)
rh(ws_dash,8,22)

bs=[
    ("Annual Income",       True, 0,                                   MONEY,INPUT_BLUE),
    ("Monthly Income",      False,"=IFERROR(B10/12,0)",                MONEY,CHARCOAL),
    ("This Month Expenses", False,f"=IFERROR(Jan!D{jan_gt},0)",        MONEY,GREEN),
    ("Left to Spend",       False,"=IFERROR(B11-B12,0)",               MONEY,CHARCOAL),
    ("Annual Savings Rate", False,"=IFERROR((B10-B12*12)/B10,0)",      PCT,  CHARCOAL),
    ("YTD Savings",         False,ytd_f,                               MONEY,GREEN),
]
ss=[
    ("Emergency Fund Goal",    True,0,                            MONEY,INPUT_BLUE),
    ("Emergency Fund Current", True,0,                            MONEY,INPUT_BLUE),
    ("% Funded",               False,"=IFERROR(F11/F10,0)",       PCT,  CHARCOAL),
    ("Monthly Target",         True,0,                            MONEY,INPUT_BLUE),
    ("YTD Saved",              False,ytd_f,                       MONEY,GREEN),
    ("Months to Goal",         False,"=IFERROR((F10-F11)/F13,0)","0.0",CHARCOAL),
]
for i in range(6):
    r=10+i
    bgb=SAGE_PALE if i%2==0 else WHITE; bgs=TERRA_LIGHT if i%2==0 else WHITE

    lbl_t,is_inp,formula,fmt,color=bs[i]
    ws_dash.merge_cells(start_row=r,start_column=1,end_row=r,end_column=2)
    sc(ws_dash,r,1,lbl_t,True,9,CHARCOAL,bgb,"left"); fill_span(ws_dash,r,1,2,bgb)
    if is_inp: inp(ws_dash,r,3,0,fmt,bgb)
    else:       frm(ws_dash,r,3,formula,fmt,color,bgb)
    ws_dash.cell(r,4).fill=fl(CREAM)

    lbl_t2,is_inp2,formula2,fmt2,color2=ss[i]
    ws_dash.merge_cells(start_row=r,start_column=5,end_row=r,end_column=6)
    sc(ws_dash,r,5,lbl_t2,True,9,CHARCOAL,bgs,"left"); fill_span(ws_dash,r,5,6,bgs)
    if is_inp2: inp(ws_dash,r,7,0,fmt2,bgs)
    else:        frm(ws_dash,r,7,formula2,fmt2,color2,bgs)
    for col in [8,9]: ws_dash.cell(r,col).fill=fl(CREAM)
    rh(ws_dash,r,18)

rh(ws_dash,16,10)
for col in range(1,10): ws_dash.cell(16,col).fill=fl(CREAM)

# Top categories + bills (rows 17-24)
mc(ws_dash,17,1,17,3,"TOP EXPENSE CATEGORIES  ·  JANUARY",True,10,WHITE,SAGE_MID,"center"); fill_span(ws_dash,17,1,3,SAGE_MID)
mc(ws_dash,17,5,17,7,"BILLS THIS MONTH",True,10,WHITE,TERRA,"center"); fill_span(ws_dash,17,5,7,TERRA)
for col in [4,8,9]: ws_dash.cell(17,col).fill=fl(CREAM)
rh(ws_dash,17,22)
for ci,h in [(1,"CATEGORY"),(2,"BUDGETED"),(3,"ACTUAL")]:
    sc(ws_dash,18,ci,h,True,9,CHARCOAL,SAGE_LIGHT,"center")
for ci,h in [(5,"BILL NAME"),(6,"AMOUNT"),(7,"DUE")]:
    sc(ws_dash,18,ci,h,True,9,CHARCOAL,TERRA_LIGHT,"center")
for col in [4,8,9]: ws_dash.cell(18,col).fill=fl(CREAM)
rh(ws_dash,18,18)

bills=[("Hyra/Mortgage",12500,"1:a"),("El",850,"15:e"),("Internet",499,"22:a"),
       ("Streaming",139,"8:e"),("Telefon",699,"12:e"),("Försäkring",1200,"1:a")]
for i,(cat,_) in enumerate(CATS[:6]):
    r=19+i; bg=SAGE_PALE if i%2==0 else WHITE; bgb=TERRA_LIGHT if i%2==0 else WHITE
    sub_r=rows["Jan"][f"{cat}_total"]
    sc(ws_dash,r,1,cat,True,9,CHARCOAL,bg,"left")
    frm(ws_dash,r,2,f"=IFERROR(Jan!C{sub_r},0)",MONEY,GREEN,bg)
    frm(ws_dash,r,3,f"=IFERROR(Jan!D{sub_r},0)",MONEY,GREEN,bg)
    ws_dash.cell(r,4).fill=fl(CREAM)
    bn,ba,bd=bills[i]
    sc(ws_dash,r,5,bn,bg=bgb,h="left"); inp(ws_dash,r,6,ba,MONEY,bgb)
    sc(ws_dash,r,7,bd,bg=bgb,h="center")
    for col in [8,9]: ws_dash.cell(r,col).fill=fl(CREAM)
    rh(ws_dash,r,18)

rh(ws_dash,25,10)
for col in range(1,10): ws_dash.cell(25,col).fill=fl(CREAM)

# ── DASHBOARD: Donut chart — expense distribution ─────────────────────────
# Helper table cols 11-12 rows 3-10 (outside visible layout)
for ji,(cat,_) in enumerate(CATS):
    sub_r=rows["Jan"][f"{cat}_total"]
    ws_dash.cell(3+ji,11).value=cat
    ws_dash.cell(3+ji,11).font=fn(9,False,CHARCOAL)
    fc=ws_dash.cell(3+ji,12,value=f"=IFERROR(Jan!D{sub_r},0)")
    fc.font=fn(9,False,CHARCOAL); fc.number_format=MONEY

donut=DoughnutChart()
donut.title="Expense Distribution — January"; donut.style=10; donut.holeSize=55
donut.height=13; donut.width=16
d_labels=Reference(ws_dash,min_col=11,min_row=3,max_row=3+len(CATS)-1)
d_values=Reference(ws_dash,min_col=12,min_row=3,max_row=3+len(CATS)-1)
donut.add_data(d_values); donut.set_categories(d_labels)
apply_chart_style(donut)
ws_dash.add_chart(donut,"A26")

# ── DASHBOARD: Annual Income vs Expenses bar chart ────────────────────────
# Helper table cols 11-13 rows 12-24
ws_dash.cell(12,11).value="Month"; ws_dash.cell(12,11).font=fn(9,True)
ws_dash.cell(12,12).value="Income"; ws_dash.cell(12,12).font=fn(9,True)
ws_dash.cell(12,13).value="Expenses"; ws_dash.cell(12,13).font=fn(9,True)
for i,mo in enumerate(MO_SHORT):
    tr=rows[mo]["income_total"]; gr=rows[mo]["grand_total"]
    ws_dash.cell(13+i,11).value=mo
    fi=ws_dash.cell(13+i,12,value=f"=IFERROR({mo}!D{tr},0)"); fi.number_format=MONEY
    fe=ws_dash.cell(13+i,13,value=f"=IFERROR({mo}!D{gr},0)"); fe.number_format=MONEY

ann_b=BarChart()
ann_b.type="col"; ann_b.grouping="clustered"; ann_b.style=10
ann_b.title="Annual Income vs Expenses"; ann_b.height=14; ann_b.width=22
a_inc=Reference(ws_dash,min_col=12,min_row=12,max_row=24)
a_exp=Reference(ws_dash,min_col=13,min_row=12,max_row=24)
a_cat=Reference(ws_dash,min_col=11,min_row=13,max_row=24)
ann_b.add_data(a_inc,titles_from_data=True)
ann_b.add_data(a_exp,titles_from_data=True)
ann_b.set_categories(a_cat)
hex_to_solidFill(ann_b.series[0],SAGE_DARK); hex_to_solidFill(ann_b.series[1],TERRA)
apply_chart_style(ann_b)
ws_dash.add_chart(ann_b,"F26")


# ════════════════════════════════════════════════════════════════════════════
# DEBT TRACKER
# ════════════════════════════════════════════════════════════════════════════
ws_debt=wb.create_sheet("Debt Tracker")
no_grid(ws_debt); ws_debt.sheet_properties.tabColor=TERRA
mc(ws_debt,1,1,1,9,"DEBT PAYOFF TRACKER",True,16,WHITE,TERRA,"center"); rh(ws_debt,1,48)
mc(ws_debt,2,1,2,9,"Avalanche = highest rate first  ·  Snowball = lowest balance first",False,9,WHITE,GOLD,"center",italic=True); rh(ws_debt,2,20)
for ci,h in enumerate(["DEBT NAME","LENDER","ORIGINAL BAL","CURRENT BAL","INTEREST %","MIN PAYMENT","MONTHLY PMT","EST PAYOFF","STATUS"],1):
    sc(ws_debt,3,ci,h,True,9,WHITE,SAGE_DARK,"center"); rh(ws_debt,3,22)
debts=["Credit Card 1","Credit Card 2","Student Loan","Car Loan","Personal Loan","Other"]
for ji,debt in enumerate(debts):
    r=4+ji; bg=TERRA_LIGHT if ji%2==0 else WHITE
    sc(ws_debt,r,1,debt,bg=bg,h="left"); inp(ws_debt,r,2,"","@",bg)
    ws_debt.cell(r,2).number_format="@"
    inp(ws_debt,r,3,0,MONEY,bg); inp(ws_debt,r,4,0,MONEY,bg)
    inp(ws_debt,r,5,0,PCT,bg); inp(ws_debt,r,6,0,MONEY,bg)
    inp(ws_debt,r,7,0,MONEY,bg); inp(ws_debt,r,8,"","DD/MM/YYYY",bg)
    sc(ws_debt,r,9,"Active",bg=bg,h="center"); rh(ws_debt,r,18)
de=4+len(debts)-1; dt=de+1
mc(ws_debt,dt,1,dt,2,"TOTALS",True,10,WHITE,TERRA,"left"); fill_span(ws_debt,dt,1,2,TERRA)
for col,f in [(3,f"=SUM(C4:C{de})"),(4,f"=SUM(D4:D{de})"),(6,f"=SUM(F4:F{de})"),(7,f"=SUM(G4:G{de})")]:
    frm(ws_debt,dt,col,f,MONEY,WHITE,TERRA)
for col in [5,8,9]: sc(ws_debt,dt,col,"",bg=TERRA)
rh(ws_debt,dt,24)
sb=dt+2
mc(ws_debt,sb,1,sb,5,"DEBT SUMMARY",True,10,WHITE,SAGE_DARK,"left",indent=1); fill_span(ws_debt,sb,1,5,SAGE_DARK); rh(ws_debt,sb,22)
for ki,(sl,sf,sfmt) in enumerate([
    ("Total Outstanding",f"=SUM(D4:D{de})",MONEY),
    ("Total Monthly Minimums",f"=SUM(F4:F{de})",MONEY),
    ("Highest Rate Debt",f'=IFERROR(INDEX(A4:A{de},MATCH(MAX(E4:E{de}),E4:E{de},0)),"-")',"@"),
    ("Debt-to-Income (monthly)",f"=IFERROR(SUM(G4:G{de})/(Dashboard!B10/12),0)",PCT),
]):
    r=sb+1+ki; bg=SAGE_PALE if ki%2==0 else WHITE
    ws_debt.merge_cells(start_row=r,start_column=1,end_row=r,end_column=3)
    sc(ws_debt,r,1,sl,True,9,CHARCOAL,bg,"left"); fill_span(ws_debt,r,1,3,bg)
    ws_debt.merge_cells(start_row=r,start_column=4,end_row=r,end_column=5)
    frm(ws_debt,r,4,sf,sfmt,GREEN,bg); fill_span(ws_debt,r,4,5,bg); rh(ws_debt,r,18)
ws_debt.freeze_panes="A3"
for ci,w in zip(range(1,10),[22,18,14,14,12,14,14,14,12]):
    ws_debt.column_dimensions[get_column_letter(ci)].width=w


# ════════════════════════════════════════════════════════════════════════════
# BILL CALENDAR
# ════════════════════════════════════════════════════════════════════════════
ws_bill=wb.create_sheet("Bill Calendar")
no_grid(ws_bill); ws_bill.sheet_properties.tabColor=TERRA
mc(ws_bill,1,1,1,9,"BILL CALENDAR  ·  2026",True,16,WHITE,TERRA,"center"); rh(ws_bill,1,48)
mc(ws_bill,2,1,2,9,"Green = paid  ·  Gold = auto-pay  ·  Never miss a payment",False,9,WHITE,GOLD,"center",italic=True); rh(ws_bill,2,20)
for ci,h in enumerate(["BILL NAME","CATEGORY","AMOUNT","DUE DATE","FREQUENCY","AUTO-PAY","PAID?","ANNUAL COST","NOTES"],1):
    sc(ws_bill,3,ci,h,True,9,WHITE,SAGE_DARK,"center"); rh(ws_bill,3,22)
bills_d=[
    ("Hyra/Mortgage","Housing",12500,"2026-01-01","Monthly","No","No"),
    ("El","Utilities",850,"2026-01-15","Monthly","No","No"),
    ("Internet","Utilities",499,"2026-01-22","Monthly","Yes","No"),
    ("Netflix/Streaming","Lifestyle",139,"2026-01-08","Monthly","Yes","No"),
    ("Telefon","Utilities",699,"2026-01-12","Monthly","Yes","No"),
    ("Hemförsäkring","Insurance",1200,"2026-01-01","Monthly","Yes","No"),
    ("Gym","Health",400,"2026-01-05","Monthly","Yes","No"),
    ("Bilförsäkring","Insurance",1100,"2026-01-20","Monthly","Yes","No"),
    ("Spotify","Lifestyle",109,"2026-01-14","Monthly","Yes","No"),
    ("A-kassa/Facket","Other",350,"2026-01-01","Monthly","Yes","No"),
]
for ji,(bn,bc,ba,bd,bf,bap,bpd) in enumerate(bills_d):
    r=4+ji; bg=TERRA_LIGHT if ji%2==0 else WHITE
    sc(ws_bill,r,1,bn,bg=bg,h="left"); sc(ws_bill,r,2,bc,bg=bg,h="left")
    inp(ws_bill,r,3,ba,MONEY,bg)
    cd=ws_bill.cell(r,4,value=bd); cd.font=fn(9,False,INPUT_BLUE); cd.fill=fl(bg)
    cd.alignment=al("right"); cd.border=tborder(); cd.number_format="DD/MM/YYYY"
    sc(ws_bill,r,5,bf,bg=bg); sc(ws_bill,r,6,bap,bg=bg,h="center"); sc(ws_bill,r,7,bpd,bg=bg,h="center")
    ac=ws_bill.cell(r,8,value=f'=IFERROR(IF(E{r}="Monthly",C{r}*12,IF(E{r}="Quarterly",C{r}*4,IF(E{r}="Annual",C{r},C{r}*52))),0)')
    ac.font=fn(9,False,CHARCOAL); ac.fill=fl(bg); ac.alignment=al("right"); ac.border=tborder(); ac.number_format=MONEY
    sc(ws_bill,r,9,"",bg=bg); rh(ws_bill,r,18)
be=4+len(bills_d)-1; bt_r=be+1
mc(ws_bill,bt_r,1,bt_r,2,"MONTHLY TOTAL",True,10,WHITE,TERRA,"left"); fill_span(ws_bill,bt_r,1,2,TERRA)
frm(ws_bill,bt_r,3,f"=SUM(C4:C{be})",MONEY,WHITE,TERRA)
frm(ws_bill,bt_r,8,f"=SUM(H4:H{be})",MONEY,WHITE,TERRA)
for col in [4,5,6,7,9]: sc(ws_bill,bt_r,col,"",bg=TERRA)
rh(ws_bill,bt_r,24)
ws_bill.conditional_formatting.add(f"A3:I{be}",
    FormulaRule(formula=['$G3="Yes"'],fill=fl(SAGE_LIGHT),font=fn(9,False,SAGE_DARK)))
ws_bill.conditional_formatting.add(f"F3:F{be}",
    FormulaRule(formula=['$F3="Yes"'],font=Font(name="Calibri",size=9,bold=True,color=GOLD)))
for dv_r,f1 in [(f"B4:B{be}",'"Housing,Utilities,Insurance,Lifestyle,Health,Transport,Other"'),
                (f"E4:E{be}",'"Monthly,Quarterly,Annual,Weekly"'),(f"F4:G{be}",'"Yes,No"')]:
    dv=DataValidation(type="list",formula1=f1,allow_blank=True)
    ws_bill.add_data_validation(dv); dv.sqref=dv_r
ws_bill.freeze_panes="A3"
for ci,w in zip(range(1,10),[22,16,12,14,12,10,8,14,22]):
    ws_bill.column_dimensions[get_column_letter(ci)].width=w


# ════════════════════════════════════════════════════════════════════════════
# SAVINGS GOALS
# ════════════════════════════════════════════════════════════════════════════
ws_sav=wb.create_sheet("Savings Goals")
no_grid(ws_sav); ws_sav.sheet_properties.tabColor=SAGE_DARK
mc(ws_sav,1,1,1,8,"SAVINGS GOALS TRACKER",True,16,WHITE,SAGE_DARK,"center"); rh(ws_sav,1,48)
mc(ws_sav,2,1,2,8,"Set targets  ·  Track progress  ·  Automate contributions",False,9,WHITE,GOLD,"center",italic=True); rh(ws_sav,2,20)
for ci,h in enumerate(["GOAL","TARGET","SAVED SO FAR","MONTHLY CONTRIB","TARGET DATE","% COMPLETE","PROGRESS","PRIORITY"],1):
    sc(ws_sav,3,ci,h,True,9,WHITE,SAGE_DARK,"center"); rh(ws_sav,3,22)
goals=[
    ("Emergency Fund (3mo)",50000,0,500,"2026-12-31","High"),
    ("Japan Trip",25000,0,200,"2026-09-01","Medium"),
    ("Home Down Payment",250000,0,800,"2029-01-01","High"),
    ("New Car",80000,0,300,"2027-06-01","Medium"),
    ("Retirement Boost",100000,0,400,"2030-01-01","High"),
    ("Education Fund",40000,0,250,"2028-01-01","Low"),
]
pb_bg={"High":TERRA_LIGHT,"Medium":SAGE_PALE,"Low":CREAM}
for ji,(gn,gt_,gc,gm,gd,gp) in enumerate(goals):
    r=4+ji; bg=pb_bg.get(gp,CREAM)
    sc(ws_sav,r,1,gn,True,9,CHARCOAL,bg,"left"); inp(ws_sav,r,2,gt_,MONEY,bg)
    inp(ws_sav,r,3,gc,MONEY,bg); inp(ws_sav,r,4,gm,MONEY,bg)
    cd=ws_sav.cell(r,5,value=gd); cd.font=fn(9,False,INPUT_BLUE); cd.fill=fl(bg)
    cd.alignment=al("right"); cd.border=tborder(); cd.number_format="DD/MM/YYYY"
    frm(ws_sav,r,6,f"=IFERROR(C{r}/B{r},0)",PCT,CHARCOAL,bg)
    pb=ws_sav.cell(r,7,value=f'=REPT(CHAR(9608),ROUND(F{r}*10,0))&REPT(CHAR(9617),10-ROUND(F{r}*10,0))')
    pb.font=Font(name="Calibri",size=11,color=SAGE_DARK); pb.fill=fl(bg); pb.alignment=al("left"); pb.border=tborder()
    sc(ws_sav,r,8,gp,True,9,CHARCOAL,bg,"center"); rh(ws_sav,r,18)
ge=4+len(goals)-1; gt_r=ge+1
for ci,v,f_ in [(1,"TOTALS","@"),(2,f"=SUM(B4:B{ge})",MONEY),(3,f"=SUM(C4:C{ge})",MONEY),
                (4,f"=SUM(D4:D{ge})",MONEY),(6,f"=IFERROR(SUM(C4:C{ge})/SUM(B4:B{ge}),0)",PCT)]:
    c=ws_sav.cell(gt_r,ci,value=v); c.font=fn(9,True,CHARCOAL); c.fill=fl(SAGE_LIGHT)
    c.alignment=al("right" if ci>1 else "left"); c.border=sage_bot(); c.number_format=f_
for ci in [5,7,8]:
    c=ws_sav.cell(gt_r,ci); c.fill=fl(SAGE_LIGHT); c.border=tborder()
rh(ws_sav,gt_r,20)
for rv,rb in [("High",TERRA_LIGHT),("Medium",SAGE_PALE),("Low",CREAM)]:
    ws_sav.conditional_formatting.add(f"H4:H{ge}",FormulaRule(formula=[f'$H4="{rv}"'],fill=fl(rb)))
dv_p=DataValidation(type="list",formula1='"High,Medium,Low"',allow_blank=True)
ws_sav.add_data_validation(dv_p); dv_p.sqref=f"H4:H{ge}"
ws_sav.freeze_panes="A3"
for ci,w in zip(range(1,9),[22,14,14,16,14,11,18,10]):
    ws_sav.column_dimensions[get_column_letter(ci)].width=w


# ════════════════════════════════════════════════════════════════════════════
# SPENDING TRACKER
# ════════════════════════════════════════════════════════════════════════════
ws_sp=wb.create_sheet("Spending Tracker")
no_grid(ws_sp); ws_sp.sheet_properties.tabColor=SAGE_DARK
mc(ws_sp,1,1,1,33,"DAYS WITHOUT SPENDING  ·  2026",True,16,WHITE,SAGE_DARK,"center"); rh(ws_sp,1,48)
mc(ws_sp,2,1,2,33,"Mark each no-spend day with X  ·  Watch your streak grow",False,9,WHITE,GOLD,"center",italic=True); rh(ws_sp,2,20)
sc(ws_sp,3,1,"MONTH",True,9,WHITE,SAGE_DARK,"center"); rh(ws_sp,3,22)
for d in range(1,32): sc(ws_sp,3,d+1,d,True,9,WHITE,SAGE_MID,"center")
sc(ws_sp,3,33,"NO-SPEND DAYS",True,9,WHITE,SAGE_DARK,"center")
days_2026=[31,28,31,30,31,30,31,31,30,31,30,31]
for mi,m_long in enumerate(MO_LONG):
    r=4+mi
    bg=SAGE_PALE if mi%2==0 else WHITE
    sc(ws_sp,r,1,m_long,True,9,CHARCOAL,SAGE_LIGHT if mi%2==0 else TERRA_LIGHT,"left"); rh(ws_sp,r,18)
    days=days_2026[mi]
    for d in range(1,32):
        c=ws_sp.cell(r,d+1)
        if d<=days:
            c.font=fn(9,False,INPUT_BLUE); c.fill=fl(bg)
            c.alignment=al("center"); c.border=tborder()
        else:
            c.fill=fl(GRAY_DIS); c.border=tborder()
    cf=ws_sp.cell(r,33,value=f'=COUNTIF(B{r}:{get_column_letter(32)}{r},"X")')
    cf.font=fn(9,True,GREEN); cf.fill=fl(SAGE_LIGHT); cf.alignment=al("center"); cf.border=tborder()
ann_sp=4+12
mc(ws_sp,ann_sp,1,ann_sp,32,"ANNUAL TOTAL NO-SPEND DAYS",True,10,WHITE,SAGE_DARK,"left",indent=1)
fill_span(ws_sp,ann_sp,1,32,SAGE_DARK)
cf_a=ws_sp.cell(ann_sp,33,value="="+"+".join([f"AG{4+mi}" for mi in range(12)]))
cf_a.font=fn(10,True,WHITE); cf_a.fill=fl(SAGE_DARK); cf_a.alignment=al("center"); cf_a.border=tborder()
rh(ws_sp,ann_sp,24)
ws_sp.conditional_formatting.add(f"B4:{get_column_letter(32)}{4+11}",
    FormulaRule(formula=['B4="X"'],fill=fl(SAGE_LIGHT),font=fn(9,True,SAGE_DARK)))
ws_sp.freeze_panes="B3"
ws_sp.column_dimensions["A"].width=14
for ci in range(2,33): ws_sp.column_dimensions[get_column_letter(ci)].width=3.5
ws_sp.column_dimensions[get_column_letter(33)].width=16


# ════════════════════════════════════════════════════════════════════════════
# SHEET ORDER CHECK + SAVE
# ════════════════════════════════════════════════════════════════════════════
expected=["Dashboard"]+MO_SHORT+["Annual Overview","Debt Tracker","Bill Calendar","Savings Goals","Spending Tracker"]
assert wb.sheetnames==expected, f"Sheet order error: {wb.sheetnames}"

OUT="ultimate_budget_final.xlsx"
wb.save(OUT)

# ════════════════════════════════════════════════════════════════════════════
# VERIFICATION
# ════════════════════════════════════════════════════════════════════════════
print(f"\n✅ KLAR! {OUT}")
print(f"   {len(wb.sheetnames)} ark\n")
for name in wb.sheetnames:
    ws_=wb[name]
    charts=len(ws_._charts)
    print(f"   {name:22s}  {ws_.max_row:3d} rader  gridlines={ws_.sheet_view.showGridLines}  charts={charts}")

print("\n📍 RADNYCKEL (Jan):")
for k,v in sorted(rows["Jan"].items()):
    print(f"   Jan.{k} = {v}")

print(f"""
📊 DIAGRAM (embedded i filen):
   Dashboard    — Donut (expense distribution Jan) @ A26
   Dashboard    — Clustered Column (income vs expenses annual) @ F26
   Annual Overview — Grouped Bar (budget vs actual) + Line (NCF trend)
   Jan–Dec (×12) — Horizontal Bar (budget vs actual per category)

⚠️  OBS: xlwings kräver desktop-Excel (Windows/Mac).
   Vi använder openpyxl-diagram istället — fungerar i alla miljöer.
   Öppna filen i Excel/Google Sheets — diagrammen är inbäddade och klara.
""")
