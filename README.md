# firmware-io-2020
Forked from firmware-io (2019). Club Robot INSA Toulouse 2020.

## IO
#### Input 
* alim
* tirette
* Capteurs fin de courses
    * 6 exemplaires (3 x 2 rails)
#### Output 
* LED 5v
    * 2 exemplaires
* Buzzer

## Affectation des pins
Les sens gauche et droite sont donnés dans le sens du robot. 
```
B9 --> Capteur fin de course gauche bas.
B8 --> Capteur fin de course gauche milieu.
B7 --> Capteur fin de course gauche haut.
B6 --> Capteur fin de course droite bas.
B5 --> Capteur fin de course droite milieu.
B4 --> Capteur fin de course droite haut.

B1 --> Tirette.
C14 --> LED Communication.
```

## Compilation
La compilation se fait avec ```cargo build --features=primary --release```. Une erreur ```linking with rust-lld failed: exit code: 1``` peut apparaitre si l'argument ```--release``` est omis.

## Lien avec le pôle info
L'API de communication est décrit dans ```librobot/src/transmission/io/mod.rs``` (plus de docs à venir).
