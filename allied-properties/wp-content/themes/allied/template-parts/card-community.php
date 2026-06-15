<?php
/**
 * Community card (used in archive + home grid).
 * Expects to be called within the loop.
 *
 * @package Allied
 */

if ( ! defined( 'ABSPATH' ) ) { exit; }

$cats     = allied_community_filter_terms( get_the_ID() );
$location = allied_project_meta( '_allied_location' );
$lots     = allied_project_meta( '_allied_lots' );
?>
<article class="card" data-cats="<?php echo esc_attr( $cats ); ?>">
	<a class="card__media" href="<?php the_permalink(); ?>" tabindex="-1" aria-hidden="true">
		<?php allied_thumbnail( 'allied-card' ); ?>
	</a>
	<div class="card__body">
		<?php
		$status = get_the_terms( get_the_ID(), 'project_status' );
		if ( $status && ! is_wp_error( $status ) ) {
			echo '<span class="card__eyebrow">' . esc_html( $status[0]->name ) . '</span>';
		}
		?>
		<h3 class="card__title"><a href="<?php the_permalink(); ?>"><?php the_title(); ?></a></h3>
		<?php if ( $location ) : ?>
			<p class="text-muted" style="margin:0;font-size:var(--fs-small);"><?php echo esc_html( $location ); ?></p>
		<?php endif; ?>
		<div class="card__meta">
			<?php if ( $lots ) : ?><span><?php echo esc_html( $lots ); ?> <?php esc_html_e( 'lots', 'allied' ); ?></span><?php endif; ?>
			<?php
			$region = get_the_terms( get_the_ID(), 'region' );
			if ( $region && ! is_wp_error( $region ) ) {
				echo '<span>' . esc_html( $region[0]->name ) . '</span>';
			}
			?>
		</div>
	</div>
</article>
