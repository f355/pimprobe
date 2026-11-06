# Kalibrera proben

Med ett fräst block kan du kalibrera både kuldiameter och alla tre förskjutningar.
Alternativt kan du använda en passbit eller mätpinne för diametern och en
vippindikator för X/Y.

## Vad kalibreringen bestämmer

### Kuldiameter

Proben ger inte signal i samma ögonblick som kulan rör vid en yta. Maskinen
hinner röra sig lite till medan mätspetsen böjs och mekanismen inuti proben
rör sig tillräckligt för att slå om de elektriska kontakterna.

PIMProbe räknar med denna extra rörelse genom en **effektiv kuldiameter**,
även kallad elektrisk kuldiameter. Den är mindre än den fysiska diametern.
En kula som är 2 mm kan till exempel behöva värdet 1,83 mm. Kalibreringen
bestämmer värdet genom att jämföra probens breddmätning med ett mått taget
oberoende av proben, till exempel med mikrometer.

PIMProbe använder halva diametern för att beräkna en berörd väggs läge och
hela diametern för måttet mellan två väggar. Med den fysiska diametern som
inställning skulle väggarnas beräknade lägen bli fel.

### X/Y-förskjutningar

Proben sitter bredvid spindeln. PIMProbe behöver avstånden i X och Y mellan
dem för att placera spindeln över en punkt som hittats med proben.

Bestäm först centrum på ett block, en tapp, ett hål eller en ficka med spindeln.
Mät sedan samma centrum med proben. Två motstående sidor ger en mittpunkt.
Kulradien påverkar sidorna lika mycket åt motsatta håll och ändrar därför
inte mittpunkten. Du kan alltså kalibrera X/Y utan att känna till kuldiametern.

### Z-förskjutning

Z-kalibreringen gör att en probad yta får samma höjd som när den frästes med
ett verktyg vars längd har mätts.

## Fräs ett block för kalibrering

Exemplet använder ett rektangulärt upphöjt block. Samma metod fungerar med
alla X/Y-centrummätningar på **Centrum**: block, tapp, hål eller ficka,
förutsatt att du har mätverktyg för att mäta de färdiga måtten noggrant.

1. Spänn fast arbetsstycket på bordet. Håll klämmorna borta från de fyra
   sidor som ska mätas.
2. Fräs ett block med centrum i X0/Y0 i valfritt arbetskoordinatsystem och
   sidorna parallella med X och Y. För Z-kalibrering, fräs ovansidan i Z0 med
   ett verktyg vars längd har mätts med verktygsmätaren. Låt arbetsstycket
   sitta kvar och behåll nollpunkten efter fräsningen.
3. Mät de färdiga måtten längs X och Y med mikrometer. Skriv ned båda.

**Pinnfräsar skär ofta med mindre diameter än den nominella.** Det färdiga
blocket kan därför avvika från programmet. Använd blockets uppmätta mått,
inte programmets mått eller diametern som står på verktyget.

## Mät blocket med proben

1. Stoppa spindeln. Ta bort spån från blockets ovansida och fyra sidor.
   Ta bort grader från kanterna där kulan ska röra vid blocket.
2. Öppna PIMProbe och välj samma arbetskoordinatsystem som vid fräsningen.
   Fäll ut proben med reglaget **Infälld / Utfälld** och rengör kulan vid behov.
3. Jogga kulan ungefär över blockets centrum. Lyft den så att hela kulan kan
   passera över blockets ovansida utan att röra vid den.
4. Öppna **Centrum** och välj **Block**.
5. Ställ **Sökavstånd X** och **Sökavstånd Y**. Vardera anger rörelsen från
   startpunkten ut åt respektive sida, innan kulan sänks. Hela kulan måste
   komma utanför blockets kant.
6. Ställ **Djup** så att kulan når de lodräta sidorna. Om kulans undersida
   börjar 5 mm över blocket och djupet är 8 mm, hamnar den 3 mm under
   ovansidan. Kulan ska träffa sidan, inte överkanten. Skaftet får inte
   träffa blocket.
7. Tryck på **Fortsätt** på granskningssidan. Proben mäter båda X-sidorna och
   båda Y-sidorna och stannar sedan ovanför det uppmätta centrumet.

Ett block som är 30 mm längs X och 20 mm längs Y kan till exempel mätas med
sökavstånd 20 mm i X och 15 mm i Y om starten är nära centrum. Kontrollera
att rörelserna utåt och nedåt går fria från övrigt material och klämmor.

## Spara blockets kalibrering

Behåll resultatet öppet. **Tryck inte på Sätt nollpunkt:** X0/Y0 är redan
det centrum som användes vid fräsningen. Om du ersätter det med den
okalibrerade probens resultat går jämförelsen för X/Y-förskjutningarna förlorad.

1. Tryck på **Kalibrera proben...** och välj **Kuldiameter**.
2. Välj **X+Y**. Ange mikrometermåtten i **Känt mått i X** och **Känt mått i Y**.
3. Sidan beräknar en kuldiameter från varje bredd. **Nytt** visar medelvärdet
   av de två diametrarna. Det är detta enda värde som sparas. Välj i stället
   **X** eller **Y** om du bara vill använda ett av måtten.
4. Tryck på **Spara kuldiameter**. Du kommer tillbaka till mätresultatet.
5. Öppna **Kalibrera proben...** igen och välj **X/Y-förskjutningar**.
6. Välj **X+Y** och tryck på **Spara X/Y-förskjutningar**. Dessa värden får
   probens centrum att stämma med det oförändrade X0/Y0 från fräsningen.
7. Stäng resultatet och mät blocket igen. Måtten ska stämma med mikrometern
   och centrum ska ligga nära X0/Y0.
8. Stäng detta resultat. Jogga kulan till några millimeter ovanför en plan
   del av blockets ovansida.
9. Välj **Centrum → Z**. Ställ **Djup** större än avståndet mellan kulan och
   ovansidan. Det anger hur långt ned proben som mest söker.
10. Tryck på **Fortsätt** på granskningssidan. Efter kontakten återgår proben
    till sin starthöjd.
11. Utan att trycka på **Sätt nollpunkt**, öppna **Kalibrera proben...** och
    tryck på **Spara Z-förskjutning**. Ovansidan frästes i Z0 och proben
    ställs nu in för att visa samma höjd.
12. Mät ovansidan igen. Den ska ligga nära Z0.

## Använd passbit eller mätpinne för kuldiametern

Du behöver inte veta var på bordet passbiten eller pinnen står. Det räcker
att känna till bredden eller diametern. Detta kalibrerar inte X/Y-förskjutningarna.

### Passbit

1. Spänn fast passbiten så att proben kommer åt båda ytorna för det kända
   måttet. Rengör dessa två ytor.
2. Rikta ytorna vinkelrätt mot X eller Y. Svep med en indikator längs en yta:
   utslaget ska vara konstant. En sned passbit ger ett för stort mått längs
   den axel som mäts.
3. Fäll ut proben och jogga kulan över passbitens mitt, tillräckligt högt för
   att passera över den.
4. Välj **Centrum → X-ribba** för mätning längs X eller **Y-ribba** för Y.
   Ställ sökavstånd och djup enligt **Mät blocket med proben**. Kulan ska
   sänkas utanför vardera sidan och sedan röra vid de två mätytorna.
5. Tryck på **Fortsätt** på granskningssidan.
6. Öppna **Kalibrera proben... → Kuldiameter** från resultatet. Ange
   passbitens kända mått och tryck på **Spara kuldiameter**.
7. Mät passbiten igen och kontrollera att bredden stämmer.

### Mätpinne

1. Spänn fast pinnen stående, parallellt med Z. Lämna tillräckligt mycket av
   den cylindriska sidan fri för att kulan ska komma åt runt hela pinnen.
   Rengör pinnen.
2. Fäll ut proben och jogga kulan ovanför pinnens ungefärliga centrum.
   Välj **Centrum → Tapp**. Sökavstånden ska få hela kulan utanför pinnen
   och djupet ska föra kulan ned till den cylindriska sidan.
3. Tryck på **Fortsätt** på granskningssidan. Efter mätningen står kulan
   ovanför det uppmätta centrumet.
4. Stäng resultatet och kör **Tapp** igen från detta läge. Den andra
   mätningen går genom centrum. En första mätning som börjar vid sidan om
   centrum kan ge ett för litet diametermått.
5. Öppna **Kalibrera proben... → Kuldiameter**, välj **X+Y** och ange pinnens
   kända diameter i båda måttfälten. Tryck på **Spara kuldiameter**.
6. Mät igen och kontrollera att båda diametrarna stämmer med pinnen.

## Använd en indikator för X/Y-förskjutningar

Använd ett fastspänt arbetsstycke med ett runt hål eller en rund tapp.
Diametern behöver inte vara känd. En vippindikator i spindeln bestämmer
centrum innan proben mäter det.

1. Sätt indikatorn i spindeln. Med spindelmotorn avstängd, vrid spindeln för
   hand och svep runt hålets vägg eller tappens utsida.
2. Justera X och Y tills indikatorn visar samma värde runt hela cirkeln.
   Spindeln är nu centrerad över hålet eller tappen.
3. Sätt X och Y till noll i valt arbetskoordinatsystem med maskinens
   koordinatreglage. Flytta inte arbetsstycket efter detta.
4. Ta bort indikatorn och fäll ut proben. Välj samma arbetskoordinatsystem
   i PIMProbe.
5. För **Hål**, jogga kulan in i hålet till den höjd som ska mätas.
   Sökavstånden ska räcka till båda väggarna. Kulan går mellan väggarna
   på denna höjd; **Djup** används inte.
6. För **Tapp**, jogga kulan ovanför centrum och ställ sökavstånd och djup
   enligt avsnittet om mätpinne.
7. Välj **Centrum → Hål** eller **Centrum → Tapp** och tryck på **Fortsätt**
   på granskningssidan.
8. Utan att trycka på **Sätt nollpunkt**, öppna
   **Kalibrera proben... → X/Y-förskjutningar**. Välj **X+Y** och tryck på
   **Spara X/Y-förskjutningar**.
9. Mät igen. Centrum ska ligga nära X0/Y0.

## När kalibreringen behöver göras om

Kalibrera efter byte av mätspetsen (skaftet med kulan), justering av probens
infästning eller en kollision som kan ha böjt eller flyttat mätspetsen.

## Återställ tidigare kalibrering

1. Öppna **Inställningar → Hjälpfunktioner → Probhistorik**.
2. Välj posten **Probkalibrering** för den ändring du vill ångra.
   **Tidigare** visar värdena före ändringen och **Sparat** visar de skrivna
   värdena.
3. Tryck på **Återställ tidigare kalibrering** och bekräfta. Kuldiametern
   och alla tre förskjutningar under **Tidigare** återställs.

Återställningen rör inte maskinen och ändrar ingen arbetsnollpunkt. Om en
nollpunkt sattes med fel kalibrering måste du mäta arbetsstycket igen.
Återställningen får en egen historikpost och kan ångras på samma sätt.
