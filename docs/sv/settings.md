# Inställningar

Inställningarna sparas när du godkänner ett värde och finns kvar efter omstart. Avstånd anges i mm; linjära matningar i mm/min.

<!-- guide-only -->
![Probinställningar](../images/settings.png)
<!-- /guide-only -->

## Kuldiameter

Kulan rör vid ytan innan probens elektriska kontakter slår om. Under den
extra rörelsen böjs mätspetsen och mekanismen inuti proben rör sig.
**Probkulans diameter** tar hänsyn till detta genom en effektiv diameter
som är mindre än den fysiska. PIMProbe använder den för att beräkna
väggarnas lägen och måtten mellan dem från de registrerade kontakterna.

Följ [Kalibrera proben](calibration.md) för att beräkna och spara värdet från
ett känt mått. Standardvärdet 1.83 mm är en utgångspunkt, inte en kalibrering.

## Tillbakagång

**Tillbakagång** är sträckan som proben backar efter varje grov- och finprobning. Den måste låta probkontakten släppa. Finsökningen går tillbaka till grovprobningens kontaktläge och får fortsätta ytterligare 0.5 mm förbi det.

## Matningar

- **Grovmatning:** den första sökningen efter en yta.
- **Finmatning:** den andra kontakten, som ger mätvärdet. Lägre hastigheter minskar probens böjning och stoppfel.
- **Matning vid flytt:** förflyttningar mellan kontakter och tillbakagång.
- **Rundaxelmatning:** rotation av A i grader/min.

## Språk

Välj engelska, förenklad kinesiska eller svenska. Tills du väljer används CNC-skärmens språk, eller datorns språk i en lokal förhandsvisning.

## Uppdateringar

**Sök uppdateringar** öppnar versionsinformationen och installationsprogrammet. **Utvecklingsversioner** väljer den löpande utvecklingsversionen i stället för stabila versioner; valet finns kvar efter installation.

**Sök automatiskt** kontrollerar den valda kanalen varje gång gränssnittet öppnas. En grön punkt på Inställningar och uppdateringsknappen betyder att en ny version finns. Installationen startar om gränssnittet.

## Hjälpfunktioner

**Probhistorik** öppnar sparade mätningar igen. **Exportera loggar** kopierar historik och diagnostikloggar till ett monterat USB-minne. **Rensa loggar** tar bort dem efter bekräftelse.

**Probens repeterbarhet** mäter L-vinkelns väggar och bordet upprepade gånger för att jämföra mätvärden, med valfri infällning av proben och referenskörning mellan körningarna.

<!-- guide-only -->
[Historik och loggar](history.md)

[Repeterbarhetskontroll](repeatability.md)

[Redigera tal](editing.md)
<!-- /guide-only -->
